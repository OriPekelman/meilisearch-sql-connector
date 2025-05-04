use crate::config::{Config, TableConfig};
use crate::database::{DatabaseAdapter, create_db_adapter};
use crate::error::{ConnectorError, Result};
use crate::meilisearch::{MeilisearchClient, MeilisearchClientTrait};
use meilisearch_sdk::settings::Settings;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{error, info, warn, debug};
use serde_json::Value;

#[derive(Clone)]
pub struct Connector {
    db_adapter: Arc<Box<dyn DatabaseAdapter>>,
    meilisearch_client: Arc<dyn MeilisearchClientTrait>,
    config: Config,
    shutdown_tx: watch::Sender<bool>,
    task_handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl Connector {
    pub async fn new(config: Config) -> Result<Self> {
        let db_url = match config.database.type_.as_str() {
            "sqlite" => {
                // Handle different path formats for SQLite
                let conn_string = &config.database.connection_string;
                
                // Check if it's a double-slash path like "//Users/..."
                if conn_string.starts_with("//") {
                    // Preserve first slash, remove second
                    let fixed_path = format!("/{}", conn_string.trim_start_matches("//"));
                    info!("Converting double-slash path to absolute path: {} -> {}", conn_string, fixed_path);
                    format!("sqlite:{}", fixed_path)
                }
                // Regular absolute path
                else if conn_string.starts_with('/') {
                    format!("sqlite:{}", conn_string)
                }
                // Path with protocol or drive letter
                else if conn_string.contains(':') {
                    format!("sqlite:{}", conn_string)
                }
                // Relative path
                else {
                    format!("sqlite:./{}", conn_string)
                }
            },
            _ => return Err(ConnectorError::UnsupportedDatabaseType(config.database.type_.clone())),
        };

        // Create database adapter with configured pool size
        let db_adapter = create_db_adapter(&db_url, Some(config.database.connection_pool_size)).await?;

        // We can add basic validation if needed using existing error types
        for table_config in &config.database.tables {
            // Get all tables from the database
            let db_tables = db_adapter.get_all_tables().await?;
            
            // Check if the specified table exists
            if !db_tables.contains(&table_config.name) {
                return Err(ConnectorError::Config(format!("Table '{}' not found in database", table_config.name)));
            }
        }

        // Create Meilisearch client
        let meilisearch_client: Arc<dyn MeilisearchClientTrait> = Arc::new(MeilisearchClient::new(
            &config.meilisearch.host,
            config.meilisearch.api_key.as_deref(),
        )?);

        // Create shutdown channel
        let (shutdown_tx, _) = watch::channel(false);

        println!("Loaded config tables: {:#?}", config.database.tables);

        Ok(Self {
            db_adapter,
            meilisearch_client,
            config,
            shutdown_tx,
            task_handles: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting connector...");
        self.setup_indices().await?;
        self.start_sync_tasks().await?;

        // Create a new receiver to keep this thread alive
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        // Wait for a shutdown signal
        info!("Connector running, press Ctrl+C to stop");
        let _ = shutdown_rx.changed().await;
        info!("Shutdown signal received, stopping connector");
        
        // Call stop to gracefully shutdown
        self.stop().await?;

        Ok(())
    }

    async fn start_sync_tasks(&self) -> Result<()> {
        if self.config.database.tables.is_empty() {
            info!("No tables configured for synchronization");
            return Ok(());
        }

        let (completion_tx, mut completion_rx) = mpsc::channel::<()>(1);
        let batch_size = self.config.database.document_batch_size;
        let max_concurrent_batches = self.config.database.max_concurrent_batches;

        // Create a receiver for each task
        for table in &self.config.database.tables {
            let poll_interval = self.config.database.poll_interval_seconds.unwrap_or(60);
            let table_clone = table.clone();
            let db_adapter = self.db_adapter.clone();
            let meilisearch_client = self.meilisearch_client.clone();
            let mut task_shutdown_rx = self.shutdown_tx.subscribe();
            let table_name = table.name.clone();
            let index_name = table.index_name.as_deref().unwrap_or(&table.name).to_string();
            let completion_tx = completion_tx.clone();
            let batch_size = batch_size;
            let max_concurrent_batches = max_concurrent_batches;
            
            // Spawn sync task
            let handle = tokio::spawn(async move {
                info!("Starting sync task for table: {}", table_name);
                
                // Initial sync
                info!("Performing initial sync for table: {}", table_name);
                match sync_table_impl(&table_clone, &index_name, &db_adapter, &meilisearch_client, batch_size, max_concurrent_batches).await {
                    Ok(_) => info!("Initial sync completed for table: {}", table_name),
                    Err(e) => error!("Error during initial sync for table {}: {}", table_name, e),
                }
                
                loop {
                    // Check if shutdown signal received
                    if *task_shutdown_rx.borrow() {
                        info!("Shutdown signal received, stopping sync for table: {}", table_name);
                        break;
                    }

                    // Sleep for the configured interval
                    tokio::select! {
                        _ = sleep(Duration::from_secs(poll_interval)) => {
                            // Continue with sync
                            info!("Polling for changes in table: {}", table_name);
                        }
                        _ = task_shutdown_rx.changed() => {
                            info!("Shutdown signal received during wait, stopping sync for table: {}", table_name);
                            break;
                        }
                    }

                    // Sync the table
                    match sync_table_impl(&table_clone, &index_name, &db_adapter, &meilisearch_client, batch_size, max_concurrent_batches).await {
                        Ok(_) => {
                            info!("Successfully synced table: {}", table_name);
                        }
                        Err(e) => {
                            error!("Error syncing table {}: {}", table_name, e);
                            // Continue loop despite error - will retry on next interval
                        }
                    }
                }
                
                // Signal task completion
                let _ = completion_tx.send(()).await;
                info!("Sync task for table {} stopped", table_name);
            });
            
            // Store handle for later joining
            self.task_handles.lock().unwrap().push(handle);
        }

        // Drop our sender so channel can close when last task completes
        drop(completion_tx);
        
        // Spawn a task to wait for completion
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        tokio::spawn(async move {
            // Wait for either all tasks to complete or shutdown signal
            tokio::select! {
                _ = async {
                    // Wait for all tasks to signal completion
                    let _ = completion_rx.recv().await;
                    info!("All sync tasks completed");
                } => {}
                _ = shutdown_rx.changed() => {
                    info!("Shutdown signal received in main sync loop");
                }
            }
        });
        
        info!("All sync tasks started");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        info!("Stopping connector...");
        
        // Send shutdown signal
        if self.shutdown_tx.send(true).is_err() {
            error!("Failed to send shutdown signal - receiver likely dropped");
        }
        
        // Wait for all tasks to complete
        let handles = {
            let mut guard = self.task_handles.lock().unwrap();
            std::mem::take(&mut *guard)
        };
        
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Error joining task: {:?}", e);
            }
        }
        
        // Allow a moment for cleanup
        sleep(Duration::from_millis(100)).await;
        
        Ok(())
    }

    async fn setup_indices(&self) -> Result<()> {
        for table in &self.config.database.tables {
            let mut settings = Settings::new();
            
            if let Some(searchable_attrs) = &table.searchable_attributes {
                settings = settings.with_searchable_attributes(searchable_attrs.iter().map(|s| s.as_str()));
            }
            if let Some(typo_tolerance) = &table.typo_tolerance {
                let mut typo_settings = meilisearch_sdk::settings::TypoToleranceSettings::default();
                typo_settings.enabled = Some(typo_tolerance.enabled);
                settings = settings.with_typo_tolerance(typo_settings);
            }
            let index_name = table.index_name.as_deref().unwrap_or(&table.name);
            
            info!("Setting up index {} with primary key {}", index_name, &table.primary_key);
            self.meilisearch_client.setup_index(index_name, settings, Some(&table.primary_key)).await?;
            
            // Wait a bit to ensure the index is created
            sleep(Duration::from_secs(1)).await;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn sync_once(&self) -> Result<()> {
        info!("Starting one-time sync...");
        self.setup_indices().await?;
        for table in &self.config.database.tables {
            let index_name = table.index_name.as_deref().unwrap_or(&table.name);
            sync_table_impl(
                table,
                index_name,
                &self.db_adapter,
                &self.meilisearch_client,
                self.config.database.document_batch_size,
                self.config.database.max_concurrent_batches,
            ).await?;
        }
        Ok(())
    }
}

fn ensure_valid_primary_key(
    doc: &Value,
    table: &TableConfig,
) -> Option<(String, Value)> {
    // Try to get the primary key value
    if let Some(id) = doc.get(&table.primary_key) {
        // Check if the ID is valid
        if id.is_null() {
            debug!("Document has null primary key: {}", table.primary_key);
            return None;
        }
        
        // Convert ID to string for mapping
        let id_str = id.to_string().trim_matches('"').to_string();
        if id_str.is_empty() || id_str == "null" || id_str == "0" {
            debug!("Document has invalid primary key value: {}", id_str);
            return None;
        }
        
        // Valid primary key found
        return Some((id_str, doc.clone()));
    } 
    
    // Primary key is missing
    debug!("Document missing primary key field: {}", table.primary_key);
    None
}

fn process_document_obj(
    table: &TableConfig,
    doc: serde_json::Map<String, Value>,
    display_id: String,
    max_text_length: usize,
    max_fields: usize,
) -> Result<Value> {
    println!("[process_document_obj] Processing doc ID: {}", display_id);
    let mut processed_doc = serde_json::Map::new();
    
    // Add the primary key
    if let Some(id_value) = doc.get(&table.primary_key) {
        debug!("Processing document with ID: {} ({:?})", display_id, id_value);
        processed_doc.insert(table.primary_key.clone(), id_value.clone());
    } else {
        return Err(ConnectorError::Config(format!("Document missing primary key: {}", table.primary_key)));
    }
    
    // Process other fields with size limits
    let mut field_count = 1; // Already counted primary key
    let mut problematic_fields = Vec::new();
    
    for (key, value) in doc {
        if key == table.primary_key {
            continue; // Already added
        }
        
        // Check if we're hitting field count limits for very wide tables
        if field_count >= max_fields {
            warn!("Document {} has too many fields, limiting to {} fields", display_id, max_fields);
            break;
        }
        
        // Check for null values or other problematic data
        if value.is_null() {
            debug!("Document {} has null value for field {}", display_id, key);
            // Use an empty string instead of null
            processed_doc.insert(key.clone(), Value::String(String::new()));
            problematic_fields.push(format!("{}=null", key));
            continue;
        }
        
        // Handle text truncation for string fields
        if let Some(text) = value.as_str() {
            if text.len() > max_text_length {
                let truncated = text.chars().take(max_text_length).collect::<String>();
                processed_doc.insert(key.clone(), Value::String(truncated));
                warn!("Truncated large text field '{}' in document {}", key, display_id);
                problematic_fields.push(format!("{}=truncated", key));
            } else {
                processed_doc.insert(key.clone(), value);
            }
        } else {
            processed_doc.insert(key.clone(), value);
        }
        
        field_count += 1;
    }
    
    // If we found problematic fields, log them
    if !problematic_fields.is_empty() {
        debug!("Document {} has problematic fields: {}", display_id, problematic_fields.join(", "));
    }
    
    // Create the final document
    let processed_value = Value::Object(processed_doc);
    
    // Check overall document size
    let serialized = serde_json::to_string(&processed_value).unwrap_or_default();
    if serialized.len() > 10_000_000 {  // 10MB max document size
        warn!("Document {} is too large ({}MB) even after processing, skipping",
              display_id, serialized.len() / 1_000_000);
        return Err(ConnectorError::Config(format!("Document too large: {} ({}MB)", display_id, serialized.len() / 1_000_000)));
    }
    
    // Return the processed document
    let result = Ok(processed_value);
    println!("[process_document_obj] Returning for ID {}: {:?}", display_id, result);
    result
}

async fn sync_table_impl(
    table: &TableConfig,
    index_name: &str,
    db_adapter: &Arc<Box<dyn DatabaseAdapter>>,
    meilisearch_client: &Arc<dyn MeilisearchClientTrait>,
    batch_size: usize,
    max_concurrent_batches: usize,
) -> Result<()> {
    info!("Syncing table {} to index {}", table.name, index_name);
    
    // Fetch documents from Meilisearch and database
    let (meili_docs, db_docs) = tokio::join!(
        meilisearch_client.get_all_documents(index_name),
        db_adapter.fetch_all_records(&table.name)
    );
    
    let meili_docs = meili_docs?;
    let db_docs = db_docs?;
    println!("[sync_table_impl] Found {} docs in DB for table '{}': {:#?}", db_docs.len(), table.name, db_docs);
    
    info!("Found {} documents in Meilisearch and {} in database", 
          meili_docs.len(), db_docs.len());

    // Extract IDs for comparison
    let mut meili_ids = std::collections::HashMap::new();
    let mut db_map = std::collections::HashMap::new();

    // Add debug information for document stats
    let mut missing_pk_count = 0;
    let mut invalid_pk_count = 0;
    let mut valid_docs = 0;

    // Build an efficient lookup map for Meilisearch documents
    for doc in &meili_docs {
        if let Some((id_str, doc_value)) = ensure_valid_primary_key(doc, table) {
            meili_ids.insert(id_str, doc_value);
        } else {
            error!("Document in Meilisearch missing valid primary key: {}", table.primary_key);
        }
    }

    // Process database documents
    for doc in &db_docs {
        if let Some((id_str, doc_value)) = ensure_valid_primary_key(doc, table) {
            db_map.insert(id_str.clone(), doc_value.clone());
            valid_docs += 1;
            debug!("[sync] Will sync doc with id: {} | doc: {:?}", id_str, doc_value);
        } else {
            // Try to identify whether it's a missing or invalid pk
            if doc.get(&table.primary_key).is_some() {
                invalid_pk_count += 1;
                if invalid_pk_count <= 5 {
                    debug!("[sync] Skipping doc with invalid primary key: {:?}", doc.get(&table.primary_key));
                }
            } else {
                missing_pk_count += 1;
                if missing_pk_count <= 5 {
                    debug!("[sync] Skipping doc missing primary key field: {} | doc: {:?}", table.primary_key, doc);
                }
            }
        }
    }

    // Log statistics about document primary keys
    if missing_pk_count > 0 || invalid_pk_count > 0 {
        warn!("Table {}: {} documents with valid primary keys, {} with invalid primary keys, {} missing primary key field", 
             table.name, valid_docs, invalid_pk_count, missing_pk_count);
    }

    println!("[sync_table_impl] DB Map Keys for '{}': {:?}", table.name, db_map.keys());
    println!("[sync_table_impl] Meili IDs Keys for '{}': {:?}", table.name, meili_ids.keys());

    // Find documents to delete (in Meilisearch but not in DB)
    let ids_to_delete: Vec<String> = meili_ids.keys()
        .filter(|id| !db_map.contains_key(*id))
        .cloned()
        .collect();

    if !ids_to_delete.is_empty() {
        info!("Deleting {} documents from index {}", ids_to_delete.len(), index_name);
        meilisearch_client.delete_documents(index_name, &ids_to_delete, Some(batch_size)).await?;
    }

    // Find documents to add or update (in DB but not in Meilisearch or modified)
    let mut documents_to_add = Vec::new();
    let max_text_length = 10000000; // Truncate text fields to this length
    let max_fields = 65536; // Limit the number of fields per document if too many

    for (id_str, doc) in db_map.iter() {
        if !meili_ids.contains_key(id_str) {
            // Document doesn't exist in Meilisearch, add it
            debug!("Adding new document with ID: {}", id_str);
            
            if let Some(obj) = doc.as_object() {
                let process_result = process_document_obj(table, obj.clone(), id_str.clone(), max_text_length, max_fields);
                println!("[sync_table_impl] Result from process_document_obj for ID {}: {:?}", id_str, process_result);
                if let Ok(processed_doc) = process_result {
                    documents_to_add.push(processed_doc);
                    println!("[sync_table_impl] Pushed doc ID {}. documents_to_add size: {}", id_str, documents_to_add.len());
                } else {
                    warn!("[sync_table_impl] Failed to process document ID {}", id_str);
                }
            } else {
                warn!("Expected document to be an object, got: {:?}", doc);
            }
        }
    }

    println!("[sync_table_impl] Checking documents_to_add before final if. Size: {}", documents_to_add.len());
    if !documents_to_add.is_empty() {
        println!("[sync_table_impl] Adding {} documents to index {}", documents_to_add.len(), index_name);
        debug!("[sync] Documents to add: {:#?}", documents_to_add);
        
        // Process documents in batches to improve performance
        let total_batches = (documents_to_add.len() + batch_size - 1) / batch_size;
        let mut batch_futures = Vec::new();
        
        for (batch_num, chunk) in documents_to_add.chunks(batch_size).enumerate() {
            let batch_num = batch_num + 1; // 1-indexed for logging
            let chunk_vec = chunk.to_vec();
            let index_name = index_name.to_string();
            let meili_client = meilisearch_client.clone();
            
            // Create a future for each batch
            let future = tokio::spawn(async move {
                info!("Processing batch {}/{} for index {}", batch_num, total_batches, index_name);
                match meili_client.add_or_update_documents(&index_name, chunk_vec, Some(batch_size)).await {
                    Ok(_) => {
                        info!("Successfully added batch {}/{} to index {}", batch_num, total_batches, index_name);
                        Ok(())
                    },
                    Err(e) => {
                        error!("Failed to add batch {}/{} to index {}: {}", batch_num, total_batches, index_name, e);
                        Err(e)
                    }
                }
            });
            
            batch_futures.push(future);
            
            // Limit concurrent batches to avoid overwhelming the Meilisearch server
            if batch_futures.len() >= max_concurrent_batches {
                // Wait for one batch to complete before adding more
                if let Some(future) = batch_futures.first_mut() {
                    println!("[sync_table_impl] Waiting for batch future to complete...");
                    let batch_result = future.await;
                    println!("[sync_table_impl] Batch future completed: {:?}", batch_result);
                    // Consider adding error handling here if needed
                }
                batch_futures.remove(0);
            }
        }
        
        // Wait for all remaining batches to complete
        for future in batch_futures {
            println!("[sync_table_impl] Waiting for remaining batch future...");
            if let Err(e) = future.await {
                error!("Error joining batch task: {:?}", e);
            }
            println!("[sync_table_impl] Remaining batch future completed.");
        }
    } else {
        println!("[sync_table_impl] No new documents to add to index {}", index_name);
    }

    Ok(())
}

use meilisearch_sdk::client::Client;
use meilisearch_sdk::settings::Settings;
use crate::error::{ConnectorError, Result};
use std::sync::Arc;
use async_trait::async_trait;
use tracing::{info, warn};
use tokio::time::{sleep, Duration};

#[async_trait]
pub trait MeilisearchClientTrait: Send + Sync {
    async fn setup_index(&self, index_name: &str, settings: Settings, primary_key: Option<&str>) -> Result<()>;
    async fn get_all_documents(&self, index_name: &str) -> Result<Vec<serde_json::Value>>;
    async fn add_or_update_documents(&self, index_name: &str, documents: Vec<serde_json::Value>, batch_size: Option<usize>) -> Result<()>;
    async fn delete_documents(&self, index_name: &str, ids: &[String], batch_size: Option<usize>) -> Result<()>;
}

pub struct MeilisearchClient {
    client: Arc<Client>,
    // Default batch sizes
    default_add_batch_size: usize,
    default_delete_batch_size: usize,
}

impl MeilisearchClient {
    pub fn new(host: &str, api_key: Option<&str>) -> Result<Self> {
        let client = Client::new(host, api_key)?;
        Ok(Self {
            client: Arc::new(client),
            default_add_batch_size: 100,
            default_delete_batch_size: 1000,
        })
    }
}

#[async_trait]
impl MeilisearchClientTrait for MeilisearchClient {
    async fn setup_index(&self, index_name: &str, settings: Settings, primary_key: Option<&str>) -> Result<()> {
        let index = self.client.index(index_name);
        
        // Create or update the index with primary key
        if let Some(pk) = primary_key {
            info!("Creating/updating index {} with primary key {}", index_name, pk);
            
            // Check if index exists first
            match self.client.get_index(index_name).await {
                Ok(_) => {
                    // Index exists, update settings
                    index.set_settings(&settings).await.map_err(ConnectorError::from)?;
                },
                Err(_) => {
                    // Index doesn't exist, create it with primary key
                    self.client.create_index(index_name, Some(pk)).await.map_err(ConnectorError::from)?;
                    
                    // Then set other settings
                    index.set_settings(&settings).await.map_err(ConnectorError::from)?;
                }
            }
        } else {
            // Just update settings if no primary key specified
            index.set_settings(&settings).await.map_err(ConnectorError::from)?;
        }
        
        // Wait a moment for settings to apply
        sleep(Duration::from_millis(500)).await;
        
        Ok(())
    }

    async fn get_all_documents(&self, index_name: &str) -> Result<Vec<serde_json::Value>> {
        let index = self.client.index(index_name);
        
        // Get documents in a single request
        // Note: This might need to be improved for very large document sets
        let result = index.get_documents().await.map_err(ConnectorError::from)?;
        
        // Log the document count
        info!("Retrieved {} documents from index {}", result.results.len(), index_name);
        
        Ok(result.results)
    }

    async fn add_or_update_documents(&self, index_name: &str, documents: Vec<serde_json::Value>, batch_size: Option<usize>) -> Result<()> {
        let batch_size = batch_size.unwrap_or(self.default_add_batch_size);
        let index = self.client.index(index_name);
        
        // Split documents into smaller batches to avoid payload size limits
        let total_docs = documents.len();
        if total_docs > batch_size {
            info!("Batching {} documents for index {} in chunks of {}", total_docs, index_name, batch_size);
        }
        
        // Debug the first document to see its structure
        if !documents.is_empty() {
            let sample_doc = &documents[0];
            info!("Sample document for {}: {}", index_name, serde_json::to_string_pretty(&sample_doc).unwrap_or_default());
        }
        
        for (i, chunk) in documents.chunks(batch_size).enumerate() {
            if total_docs > batch_size {
                info!("Processing batch {}/{} for index {}", 
                     i + 1, (total_docs + batch_size - 1) / batch_size, index_name);
            }
            
            // Process the batch
            match index.add_documents(chunk, None).await {
                Ok(_) => {
                    // Log success but don't wait for task completion
                    // This avoids compatibility issues with different versions of the SDK
                    if total_docs > batch_size {
                        info!("Successfully submitted batch {}/{} to index {}", 
                            i + 1, (total_docs + batch_size - 1) / batch_size, index_name);
                    }
                },
                Err(e) => {
                    warn!("Error adding batch {}/{} to index {}: {}", 
                         i + 1, (total_docs + batch_size - 1) / batch_size, index_name, e);
                    // Log a sample document for debugging
                    if !chunk.is_empty() {
                        warn!("Sample document in failed batch: {}", 
                            serde_json::to_string(&chunk[0]).unwrap_or_default());
                    }
                    return Err(ConnectorError::from(e));
                }
            }
            
            // Small delay between batches to avoid overwhelming the server
            if i < documents.chunks(batch_size).count() - 1 {
                sleep(Duration::from_millis(100)).await;
            }
        }
        
        Ok(())
    }

    async fn delete_documents(&self, index_name: &str, ids: &[String], batch_size: Option<usize>) -> Result<()> {
        let batch_size = batch_size.unwrap_or(self.default_delete_batch_size);
        let index = self.client.index(index_name);
        
        // Split document IDs into smaller batches
        let total_ids = ids.len();
        if total_ids > batch_size {
            info!("Batching {} document deletions for index {} in chunks of {}", total_ids, index_name, batch_size);
        }
        
        for (i, chunk) in ids.chunks(batch_size).enumerate() {
            match index.delete_documents(chunk).await {
                Ok(_) => {
                    if total_ids > batch_size {
                        info!("Successfully deleted batch {}/{} from index {}", 
                            i + 1, (total_ids + batch_size - 1) / batch_size, index_name);
                    }
                },
                Err(e) => {
                    warn!("Error deleting batch {}/{} from index {}: {}", 
                          i + 1, (total_ids + batch_size - 1) / batch_size, index_name, e);
                    return Err(ConnectorError::from(e));
                }
            }
            
            // Small delay between batches to avoid overwhelming the server
            if i < ids.chunks(batch_size).count() - 1 {
                sleep(Duration::from_millis(100)).await;
            }
        }
        
        Ok(())
    }
} 
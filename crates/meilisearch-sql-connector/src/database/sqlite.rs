use crate::database::DatabaseAdapter;
use crate::error::{ConnectorError, Result};
use sqlx::{Column, Row, SqlitePool, pool::PoolOptions, sqlite::SqliteRow};
use serde_json::{Value, Map};
use tracing::{info, debug, warn};
use std::path::Path;

pub struct SqliteAdapter {
    pool: SqlitePool,
    #[allow(dead_code)]
    path: String,
}

impl SqliteAdapter {
    pub async fn new(path: &str) -> Result<Self> {
        // For debug purposes
        debug!("SQLite adapter initializing with path: {}", path);
        
        // Normalize path - handle double slashes at the beginning
        let normalized_path = if path.starts_with("//") {
            // Convert //Users/... to /Users/... (absolute path)
            format!("/{}", path.trim_start_matches("//"))
        } else {
            path.to_string()
        };
        
        debug!("Normalized path: {}", normalized_path);
        
        // Verify path exists for file-based databases
        if normalized_path != ":memory:" {
            let file_path = Path::new(&normalized_path);
            if !file_path.exists() && file_path.parent().map_or(false, |p| p.exists()) {
                debug!("SQLite database file does not exist but will be created: {}", normalized_path);
            } else if !file_path.exists() {
                debug!("SQLite database path does not exist: {}", normalized_path);
                // Check if the path without sqlite: prefix exists
                if normalized_path.starts_with("sqlite:") {
                    let stripped = normalized_path.trim_start_matches("sqlite:");
                    if Path::new(stripped).exists() {
                        debug!("Path exists without sqlite: prefix: {}", stripped);
                    }
                }
            }
        }
        
        // Create connection string - handle sqlite:// correctly
        let connection_string = if normalized_path == ":memory:" {
            // For in-memory database
            "sqlite::memory:".to_string()
        } else if normalized_path.starts_with("sqlite://") {
            // Already in correct format for SQLx
            normalized_path.clone()
        } else if normalized_path.starts_with("sqlite:") {
            // Already has the prefix
            normalized_path.clone()
        } else if normalized_path.starts_with('/') {
            // Absolute path
            format!("sqlite://{}", normalized_path)
        } else if normalized_path.contains(':') {
            // Path already has a protocol or drive letter (Windows)
            format!("sqlite:{}", normalized_path)
        } else {
            // Relative path - ensure it has proper format
            format!("sqlite:./{}", normalized_path)
        };
        
        debug!("SQLite connection string: {}", connection_string);
        eprintln!("[SqliteAdapter] Connecting with string: {}", connection_string);
        if normalized_path != ":memory:" {
            eprintln!("[SqliteAdapter] File exists at {}: {}", normalized_path, std::path::Path::new(&normalized_path).exists());
        }
        
        // Set up connection pool with default pool size (will be overridden when used by connector)
        let pool = PoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await
            .map_err(|e| ConnectorError::Database(format!("Failed to connect to SQLite database at {}: {}", normalized_path, e)))?;
        
        info!("Connected to SQLite database at {}", normalized_path);
        
        Ok(Self {
            pool,
            path: normalized_path,
        })
    }

    // Add method to create with specific pool size
    pub async fn new_with_pool_size(path: &str, pool_size: u32) -> Result<Self> {
        // For debug purposes
        debug!("SQLite adapter initializing with path: {} and pool size: {}", path, pool_size);
        
        // Normalize path - handle double slashes at the beginning
        let normalized_path = if path.starts_with("//") {
            // Convert //Users/... to /Users/... (absolute path)
            format!("/{}", path.trim_start_matches("//"))
        } else {
            path.to_string()
        };
        
        debug!("Normalized path: {}", normalized_path);
        
        // Verify path exists for file-based databases
        if normalized_path != ":memory:" {
            let file_path = Path::new(&normalized_path);
            if !file_path.exists() && file_path.parent().map_or(false, |p| p.exists()) {
                debug!("SQLite database file does not exist but will be created: {}", normalized_path);
            } else if !file_path.exists() {
                debug!("SQLite database path does not exist: {}", normalized_path);
                // Check if the path without sqlite: prefix exists
                if normalized_path.starts_with("sqlite:") {
                    let stripped = normalized_path.trim_start_matches("sqlite:");
                    if Path::new(stripped).exists() {
                        debug!("Path exists without sqlite: prefix: {}", stripped);
                    }
                }
            }
        }
        
        // Create connection string - handle sqlite:// correctly
        let connection_string = if normalized_path == ":memory:" {
            // For in-memory database
            "sqlite::memory:".to_string()
        } else if normalized_path.starts_with("sqlite://") {
            // Already in correct format for SQLx
            normalized_path.clone()
        } else if normalized_path.starts_with("sqlite:") {
            // Already has the prefix
            normalized_path.clone()
        } else if normalized_path.starts_with('/') {
            // Absolute path
            format!("sqlite://{}", normalized_path)
        } else if normalized_path.contains(':') {
            // Path already has a protocol or drive letter (Windows)
            format!("sqlite:{}", normalized_path)
        } else {
            // Relative path - ensure it has proper format
            format!("sqlite:./{}", normalized_path)
        };
        
        debug!("SQLite connection string: {}", connection_string);
        eprintln!("[SqliteAdapter] Connecting with string: {}", connection_string);
        if normalized_path != ":memory:" {
            eprintln!("[SqliteAdapter] File exists at {}: {}", normalized_path, std::path::Path::new(&normalized_path).exists());
        }
        
        // Set up connection pool with specified pool size
        let pool = PoolOptions::new()
            .max_connections(pool_size)
            .connect(&connection_string)
            .await
            .map_err(|e| ConnectorError::Database(format!("Failed to connect to SQLite database at {}: {}", normalized_path, e)))?;
        
        info!("Connected to SQLite database at {} with connection pool size {}", normalized_path, pool_size);
        
        Ok(Self {
            pool,
            path: normalized_path,
        })
    }
    
    fn row_to_json(&self, row: SqliteRow) -> Value {
        let mut map = Map::new();
        
        // Get column names
        for (i, column) in row.columns().iter().enumerate() {
            let column_name = column.name();
            
            // First try to get the value as different types
            let value = if let Ok(val) = row.try_get::<i64, _>(i) {
                // Special handling for primary key values - ensure they're never null
                if column_name == "id" {
                    if val == 0 {
                        debug!("Found id with value 0, converting to proper number");
                    }
                    // Always ensure the ID is a proper number
                    Value::Number(val.into())
                } else {
                    Value::Number(val.into())
                }
            } else if let Ok(val) = row.try_get::<f64, _>(i) {
                // Convert f64 to serde_json::Number
                if let Some(num) = serde_json::Number::from_f64(val) {
                    Value::Number(num)
                } else {
                    Value::Null
                }
            } else if let Ok(val) = row.try_get::<String, _>(i) {
                Value::String(val)
            } else if let Ok(val) = row.try_get::<bool, _>(i) {
                Value::Bool(val)
            } else if let Ok(val) = row.try_get::<Vec<u8>, _>(i) {
                Value::String(format!("BLOB({})", val.len()))
            } else if row.try_get::<Option<String>, _>(i).is_ok() {
                // Column is null
                if column_name == "id" {
                    // For ID columns, replace null with 0 to avoid issues
                    debug!("Found null id, using 0 instead");
                    Value::Number(0.into())
                } else {
                    Value::Null
                }
            } else {
                // Default to null if we can't determine the type
                warn!("Could not determine type of column {}", column_name);
                if column_name == "id" {
                    // For ID columns, use 0 as a fallback
                    debug!("Using fallback 0 for id with undetermined type");
                    Value::Number(0.into())
                } else {
                    Value::Null
                }
            };
            
            map.insert(column_name.to_string(), value);
        }
        
        Value::Object(map)
    }
}

#[async_trait::async_trait]
impl DatabaseAdapter for SqliteAdapter {
    async fn fetch_all_records(&self, table: &str) -> Result<Vec<Value>> {
        let query = format!("SELECT * FROM {}", table);
        debug!("Executing query: {}", query);
        
        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ConnectorError::Database(format!("Failed to fetch records: {}", e)))?;
        
        let results = rows.into_iter()
            .map(|row| self.row_to_json(row))
            .collect();
        
        Ok(results)
    }

    async fn get_all_tables(&self) -> Result<Vec<String>> {
        let query = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'";
        debug!("Executing query: {}", query);
        
        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ConnectorError::Database(format!("Failed to get tables: {}", e)))?;
        
        let results = rows.into_iter()
            .map(|row| row.try_get("name"))
            .collect::<std::result::Result<Vec<String>, _>>()
            .map_err(|e| ConnectorError::Database(format!("Failed to extract table names: {}", e)))?;
        
        Ok(results)
    }

    async fn get_table_columns(&self, table: &str) -> Result<Vec<(String, String, bool)>> {
        let query = format!("PRAGMA table_info({})", table);
        debug!("Executing query: {}", query);
        
        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ConnectorError::Database(format!("Failed to get table columns: {}", e)))?;
        
        let mut results = Vec::new();
        for row in rows {
            let name: String = row.try_get("name")
                .map_err(|e| ConnectorError::Database(format!("Failed to get column name: {}", e)))?;
            
            let type_: String = row.try_get("type")
                .map_err(|e| ConnectorError::Database(format!("Failed to get column type: {}", e)))?;
            
            let pk: i64 = row.try_get("pk")
                .map_err(|e| ConnectorError::Database(format!("Failed to get primary key flag: {}", e)))?;
            
            results.push((name, type_, pk == 1));
        }
        
        Ok(results)
    }

    async fn get_primary_key(&self, table: &str) -> Result<String> {
        let query = format!("PRAGMA table_info({})", table);
        debug!("Executing query: {}", query);
        
        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ConnectorError::Database(format!("Failed to get table info: {}", e)))?;
        
        for row in rows {
            let pk: i64 = row.try_get("pk")
                .map_err(|e| ConnectorError::Database(format!("Failed to get primary key flag: {}", e)))?;
            
            if pk == 1 {
                let name: String = row.try_get("name")
                    .map_err(|e| ConnectorError::Database(format!("Failed to get column name: {}", e)))?;
                
                return Ok(name);
            }
        }
        
        Err(ConnectorError::NoPrimaryKey(table.to_string()))
    }
}

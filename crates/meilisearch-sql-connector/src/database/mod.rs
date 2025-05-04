use serde_json::Value;
use std::sync::Arc;

use crate::error::Result;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "mysql")]
pub mod mysql;

// Database adapter trait
#[async_trait::async_trait]
pub trait DatabaseAdapter: Send + Sync {
    /// Fetch all records from a table
    async fn fetch_all_records(&self, table: &str) -> Result<Vec<Value>>;
    
    /// Get all tables in the database
    async fn get_all_tables(&self) -> Result<Vec<String>>;
    
    /// Get all columns in a table (name, type, is_primary_key)
    async fn get_table_columns(&self, table: &str) -> Result<Vec<(String, String, bool)>>;
    
    /// Get the primary key of a table
    async fn get_primary_key(&self, table: &str) -> Result<String>;
}

// Database URL parser and connection factory
pub async fn create_db_adapter(url: &str, pool_size: Option<u32>) -> Result<Arc<Box<dyn DatabaseAdapter>>> {
    let parsed_url = url::Url::parse(url).map_err(|e| {
        crate::error::ConnectorError::Config(format!("Invalid database URL: {}", e))
    })?;
    
    let adapter: Box<dyn DatabaseAdapter> = match parsed_url.scheme() {
        #[cfg(feature = "sqlite")]
        "sqlite" => {
            let path = parsed_url.path();
            if let Some(size) = pool_size {
                Box::new(sqlite::SqliteAdapter::new_with_pool_size(path, size).await?)
            } else {
                Box::new(sqlite::SqliteAdapter::new(path).await?)
            }
        },
        #[cfg(feature = "postgres")]
        "postgres" | "postgresql" => {
            Box::new(postgres::PostgresAdapter::new(url).await?)
        },
        #[cfg(feature = "mysql")]
        "mysql" => {
            Box::new(mysql::MySqlAdapter::new(url).await?)
        },
        scheme => return Err(crate::error::ConnectorError::UnsupportedDatabaseType(scheme.to_string())),
    };
    
    Ok(Arc::new(adapter))
}

// Conditional exports based on enabled features

#[cfg(feature = "postgres")]
pub use postgres::PostgresAdapter;

#[cfg(feature = "mysql")]
pub use mysql::MySqlAdapter;

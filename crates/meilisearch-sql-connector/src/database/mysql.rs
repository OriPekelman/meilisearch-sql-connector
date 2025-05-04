use crate::database::DatabaseAdapter;
use crate::error::{ConnectorError, Result};
use serde_json::Value;
use async_trait::async_trait;
use tracing::{info, debug};

pub struct MySqlAdapter {
    connection_string: String,
}

impl MySqlAdapter {
    pub async fn new(connection_string: &str) -> Result<Self> {
        info!("MySQL adapter is currently a stub implementation");
        Ok(Self {
            connection_string: connection_string.to_string(),
        })
    }
}

#[async_trait]
impl DatabaseAdapter for MySqlAdapter {
    async fn fetch_all_records(&self, table: &str) -> Result<Vec<Value>> {
        debug!("MySQL stub: fetch_all_records called for table {}", table);
        Err(ConnectorError::NotImplemented("MySQL adapter fetch_all_records".to_string()))
    }
    
    async fn fetch_record(&self, table: &str, id: &str) -> Result<Value> {
        debug!("MySQL stub: fetch_record called for table {}, id {}", table, id);
        Err(ConnectorError::NotImplemented("MySQL adapter fetch_record".to_string()))
    }
    
    async fn get_table_schema(&self, table: &str) -> Result<Vec<(String, String)>> {
        debug!("MySQL stub: get_table_schema called for table {}", table);
        Err(ConnectorError::NotImplemented("MySQL adapter get_table_schema".to_string()))
    }
    
    async fn get_all_tables(&self) -> Result<Vec<String>> {
        debug!("MySQL stub: get_all_tables called");
        Err(ConnectorError::NotImplemented("MySQL adapter get_all_tables".to_string()))
    }
    
    async fn get_table_columns(&self, table: &str) -> Result<Vec<(String, String, bool)>> {
        debug!("MySQL stub: get_table_columns called for table {}", table);
        Err(ConnectorError::NotImplemented("MySQL adapter get_table_columns".to_string()))
    }
    
    async fn get_primary_key(&self, table: &str) -> Result<String> {
        debug!("MySQL stub: get_primary_key called for table {}", table);
        Err(ConnectorError::NotImplemented("MySQL adapter get_primary_key".to_string()))
    }
} 
use meilisearch_sdk::settings::Settings;
use async_trait::async_trait;
use serde_json::Value;

use crate::{
    error::Result,
    meilisearch::MeilisearchClientTrait,
    database::DatabaseAdapter,
};

// --- Mock Meilisearch Client ---
pub struct MockMeilisearchClient;

impl MockMeilisearchClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MeilisearchClientTrait for MockMeilisearchClient {
    async fn setup_index(&self, _index_name: &str, _settings: Settings, _primary_key: Option<&str>) -> Result<()> {
        Ok(())
    }

    async fn get_all_documents(&self, _index_name: &str) -> Result<Vec<Value>> {
        Ok(vec![])
    }

    async fn add_or_update_documents(&self, _index_name: &str, _documents: Vec<Value>) -> Result<()> {
        Ok(())
    }

    async fn delete_documents(&self, _index_name: &str, _ids: &[String]) -> Result<()> {
        Ok(())
    }
}

// --- Non-mockall implementation for MockSqliteAdapter ---
#[cfg(not(feature = "mockall"))]
pub struct MockSqliteAdapter;

#[cfg(not(feature = "mockall"))]
impl MockSqliteAdapter {
    pub async fn new() -> Self {
        Self
    }
}

#[cfg(not(feature = "mockall"))]
#[async_trait]
impl DatabaseAdapter for MockSqliteAdapter {
    async fn get_all_tables(&self) -> Result<Vec<String>> {
        Ok(vec!["test".to_string()])
    }

    async fn get_table_columns(&self, _table: &str) -> Result<Vec<(String, String, bool)>> {
        Ok(vec![("id".to_string(), "INTEGER".to_string(), true)])
    }

    async fn get_primary_key(&self, _table: &str) -> Result<String> {
        Ok("id".to_string())
    }

    async fn fetch_all_records(&self, _table: &str) -> Result<Vec<Value>> {
        Ok(vec![])
    }

    async fn fetch_record(&self, _table: &str, _id: &str) -> Result<Value> {
        Ok(Value::Null)
    }

    async fn get_table_schema(&self, _table: &str) -> Result<Vec<(String, String)>> {
        Ok(vec![("id".to_string(), "INTEGER".to_string())])
    }
}

#[cfg(feature = "mockall")]
pub mod mock_db {
    use mockall::mock;
    use async_trait::async_trait;
    use serde_json::Value;
    use crate::{
        error::Result,
        database::DatabaseAdapter,
    };

    // --- Mock Database Adapter ---
    mock! {
        pub MockSqliteAdapter {}

        #[async_trait]
        impl DatabaseAdapter for MockSqliteAdapter {
            async fn get_all_tables(&self) -> Result<Vec<String>>;
            async fn get_table_columns(&self, table: &str) -> Result<Vec<(String, String, bool)>>;
            async fn get_primary_key(&self, table: &str) -> Result<String>;
            async fn fetch_all_records(&self, table: &str) -> Result<Vec<Value>>;
            async fn fetch_record(&self, table: &str, id: &str) -> Result<Value>;
            async fn get_table_schema(&self, table: &str) -> Result<Vec<(String, String)>>;
        }
    }
}

#[cfg(feature = "mockall")]
pub use mock_db::MockMockSqliteAdapter as MockSqliteAdapter; 
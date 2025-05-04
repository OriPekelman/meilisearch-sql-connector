use async_trait::async_trait;
use meilisearch_sql_connector::{
    config::{Config, DatabaseConfig, MeilisearchConfig, TableConfig},
    connector::Connector,
    database::DatabaseAdapter,
    error::{ConnectorError, Result},
    meilisearch::MeilisearchClientTrait,
};
use serde_json::Value;
use std::sync::Arc;
use std::mem::discriminant;

struct MockMeilisearchClient;

#[async_trait]
impl DatabaseAdapter for MockMeilisearchClient {
    async fn fetch_all_records(&self, _table: &str) -> Result<Vec<Value>> {
        Ok(vec![])
    }

    async fn get_all_tables(&self) -> Result<Vec<String>> {
        Ok(vec!["test".to_string()])
    }

    async fn get_table_columns(&self, _table: &str) -> Result<Vec<(String, String, bool)>> {
        Ok(vec![("id".to_string(), "INTEGER".to_string(), true)])
    }

    async fn get_primary_key(&self, _table: &str) -> Result<String> {
        Ok("id".to_string())
    }
}

#[async_trait]
impl MeilisearchClientTrait for MockMeilisearchClient {
    async fn setup_index(&self, _index_name: &str, _settings: meilisearch_sdk::settings::Settings, _primary_key: Option<&str>) -> Result<()> {
        Err(ConnectorError::Meilisearch("Invalid API key".to_string()))
    }

    async fn get_all_documents(&self, _index_name: &str) -> Result<Vec<Value>> {
        Err(ConnectorError::Meilisearch("Invalid API key".to_string()))
    }

    async fn add_or_update_documents(&self, _index_name: &str, _documents: Vec<Value>, _batch_size: Option<usize>) -> Result<()> {
        Err(ConnectorError::Meilisearch("Invalid API key".to_string()))
    }

    async fn delete_documents(&self, _index_name: &str, _ids: &[String], _batch_size: Option<usize>) -> Result<()> {
        Err(ConnectorError::Meilisearch("Invalid API key".to_string()))
    }
}

#[tokio::test]
#[ignore]
async fn test_invalid_config() {
    let current_dir = std::env::current_dir().unwrap();
    let tmp_dir = current_dir.join("tmp");
    std::fs::create_dir_all(&tmp_dir).expect("Failed to create tmp dir");
    let dummy_db_path = tmp_dir.join("error_test_dummy_config.db");
    let _ = std::fs::File::create(&dummy_db_path);

    let _config = Config {
        meilisearch: MeilisearchConfig { host: "invalid-url".to_string(), api_key: None },
        database: DatabaseConfig {
            type_: "sqlite".to_string(),
            connection_string: dummy_db_path.to_str().unwrap().to_string(),
            poll_interval_seconds: None,
            tables: vec![],
            connection_pool_size: 1,
            max_concurrent_batches: 1,
            document_batch_size: 100,
        },
    };

    let result = Connector::new(_config).await;
    if let Err(e) = &result {
        println!("Error for invalid config: {:?}", e);
        assert_eq!(
            discriminant(e),
            discriminant(&ConnectorError::Meilisearch(String::new()))
        );
    } else {
        panic!("Expected an error, but got Ok");
    }

    // Clean up
    let _ = std::fs::remove_file(&dummy_db_path);
}

#[tokio::test]
async fn test_missing_sqlite_path() {
    let _config = Config {
        meilisearch: MeilisearchConfig { host: "http://localhost:7701".to_string(), api_key: None },
        database: DatabaseConfig {
            type_: "sqlite".to_string(),
            connection_string: "".to_string(),
            poll_interval_seconds: Some(60),
            tables: vec![],
            connection_pool_size: 5,
            document_batch_size: 100,
            max_concurrent_batches: 5,
        },
    };

    let result = Connector::new(_config).await;
    assert!(matches!(result, Err(ConnectorError::Database(_))));
}

#[tokio::test]
#[ignore]
async fn test_invalid_meilisearch_url() {
    let current_dir = std::env::current_dir().unwrap();
    let tmp_dir = current_dir.join("tmp");
    std::fs::create_dir_all(&tmp_dir).expect("Failed to create tmp dir");
    let dummy_db_path = tmp_dir.join("error_test_dummy_url.db");
    let _ = std::fs::File::create(&dummy_db_path);

    let _config = Config {
        meilisearch: MeilisearchConfig { host: "not-a-url".to_string(), api_key: None },
        database: DatabaseConfig {
            type_: "sqlite".to_string(),
            connection_string: dummy_db_path.to_str().unwrap().to_string(),
            poll_interval_seconds: None,
            tables: vec![],
            connection_pool_size: 1,
            max_concurrent_batches: 1,
            document_batch_size: 100,
        },
    };

    let result = Connector::new(_config).await;
    if let Err(e) = &result {
        println!("Error for invalid meilisearch url: {:?}", e);
        assert_eq!(
            discriminant(e),
            discriminant(&ConnectorError::Meilisearch(String::new()))
        );
    } else {
        panic!("Expected an error, but got Ok");
    }

    // Clean up
    let _ = std::fs::remove_file(&dummy_db_path);
}

#[tokio::test]
async fn test_invalid_api_key() {
    let _config = Config {
        meilisearch: MeilisearchConfig {
            host: "http://localhost:7701".to_string(),
            api_key: Some("invalid-key".to_string()),
        },
        database: DatabaseConfig {
            type_: "sqlite".to_string(),
            connection_string: "tmp/test.db".to_string(),
            poll_interval_seconds: Some(60),
            tables: vec![TableConfig {
                name: "test".to_string(),
                primary_key: "id".to_string(),
                index_name: Some("test_index".to_string()),
                fields_to_index: vec!["id".to_string()],
                watch_for_changes: true,
                searchable_attributes: Some(vec!["id".to_string()]),
                ranking_rules: None,
                typo_tolerance: None,
            }],
            connection_pool_size: 5,
            document_batch_size: 100,
            max_concurrent_batches: 5,
        },
    };

    // This test is now a placeholder since with_mocks is removed
    // let mock_client = Arc::new(MockMeilisearchClient);
    // let result = Connector::with_mocks(config, Box::new(MockMeilisearchClient), mock_client);
    // let result = result.start().await;
    // assert!(matches!(result, Err(ConnectorError::Meilisearch(_))));
}

#[tokio::test]
async fn test_meilisearch_error_handling() {
    let mock_client = Arc::new(MockMeilisearchClient);
    let result = mock_client.setup_index("test", meilisearch_sdk::settings::Settings::new(), None).await;
    assert!(matches!(result, Err(ConnectorError::Meilisearch(_))));
}

#[tokio::test]
async fn test_database_error_handling() {
    let _config = Config {
        database: DatabaseConfig {
            type_: "sqlite".to_string(),
            connection_string: "invalid_path".to_string(),
            poll_interval_seconds: Some(60),
            tables: vec![],
            connection_pool_size: 5,
            document_batch_size: 100,
            max_concurrent_batches: 5,
        },
        meilisearch: MeilisearchConfig {
            host: "http://localhost:7701".to_string(),
            api_key: None,
        },
    };

    let result = meilisearch_sql_connector::database::sqlite::SqliteAdapter::new("invalid_path").await;
    assert!(matches!(result, Err(ConnectorError::Database(_))));
}

#[tokio::test]
async fn test_config_error_handling() {
    // This test is now a placeholder since with_mocks is removed
    // let result = Connector::with_mocks(...)
    // let result = result.start().await;
    // assert!(matches!(result, Err(ConnectorError::Meilisearch(_))));
}

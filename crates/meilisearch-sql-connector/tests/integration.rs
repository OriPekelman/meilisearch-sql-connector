use meilisearch_sql_connector::database::{sqlite::SqliteAdapter, DatabaseAdapter};
use meilisearch_sql_connector::error::{Result, ConnectorError};
use sqlx::{Connection, SqliteConnection};
use std::fs;

/// Test with a file-based database in a temporary directory
#[tokio::test]
async fn test_basic_integration() -> Result<()> {
    // Create absolute path to tmp directory
    let current_dir = std::env::current_dir().unwrap();
    let tmp_dir = current_dir.join("tmp");
    fs::create_dir_all(&tmp_dir).unwrap();
    
    // Create a database file in the project tmp directory with absolute path
    let db_path = tmp_dir.join(format!("integration_test_{}.db", std::process::id()));
    let db_path_str = db_path.to_str().unwrap();
    println!("Using database at: {}", db_path_str);
    
    // Remove the file if it exists
    if db_path.exists() {
        std::fs::remove_file(&db_path).unwrap();
    }
    // Ensure the file exists before connecting
    std::fs::File::create(&db_path)?;
    
    // Connect to SQLite database
    let conn_str = format!("sqlite://{}", db_path_str);
    println!("Connection string: {}", conn_str);
    
    let mut conn = SqliteConnection::connect(&conn_str).await
        .map_err(|e| ConnectorError::Database(format!("Failed to connect: {}", e)))?;
    
    // Create a test table
    sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
        .execute(&mut conn)
        .await
        .map_err(|e| ConnectorError::Database(format!("Failed to create table: {}", e)))?;
    
    // Insert a test record
    sqlx::query("INSERT INTO test (id, name) VALUES (1, 'Test Item')")
        .execute(&mut conn)
        .await
        .map_err(|e| ConnectorError::Database(format!("Failed to insert data: {}", e)))?;
    
    // Close connection
    drop(conn);
    
    // Connect to the database with our adapter
    let adapter = SqliteAdapter::new(db_path_str).await?;
    
    // Test if we can retrieve the tables
    let tables = adapter.get_all_tables().await?;
    assert!(tables.contains(&"test".to_string()), "Test table not found");
    
    // Test if we can get the primary key
    let pk = adapter.get_primary_key("test").await?;
    assert_eq!(pk, "id", "Expected primary key to be 'id'");
    
    // Test if we can retrieve records
    let records = adapter.fetch_all_records("test").await?;
    assert_eq!(records.len(), 1, "Expected one record");
    assert_eq!(records[0].get("id").unwrap().as_i64().unwrap(), 1);
    assert_eq!(records[0].get("name").unwrap().as_str().unwrap(), "Test Item");
    
    // Clean up the file
    std::fs::remove_file(&db_path).unwrap_or_default();
    
    Ok(())
}

#[tokio::test]
async fn test_mock_integration() -> Result<()> {
    // Create a simple mock adapter within the test
    struct MockAdapter;
    
    #[async_trait::async_trait]
    impl DatabaseAdapter for MockAdapter {
        async fn fetch_all_records(&self, _table: &str) -> Result<Vec<serde_json::Value>> {
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
    
    let adapter = MockAdapter;
    
    // Test the adapter methods through our common interface
    let tables = adapter.get_all_tables().await?;
    assert!(tables.contains(&"test".to_string()), "Test table not found");
    
    let pk = adapter.get_primary_key("test").await?;
    assert_eq!(pk, "id", "Primary key should be 'id'");
    
    let records = adapter.fetch_all_records("test").await?;
    assert!(records.is_empty(), "Mock should return empty records list");
    
    Ok(())
}

use meilisearch_sql_connector::{
    database::{DatabaseAdapter, sqlite::SqliteAdapter},
    error::Result,
};
use sqlx::{Connection, SqliteConnection};
use std::fs;
use async_trait::async_trait;
use serde_json::Value;

#[tokio::test]
async fn test_sqlite_adapter() -> Result<()> {
    // Create absolute path to tmp directory
    let current_dir = std::env::current_dir().unwrap();
    let tmp_dir = current_dir.join("tmp");
    fs::create_dir_all(&tmp_dir).unwrap();
    
    // Create a database file in the project tmp directory with absolute path
    let db_path = tmp_dir.join(format!("sqlite_test_{}.db", std::process::id()));
    let db_path_str = db_path.to_str().unwrap();
    println!("Using database at: {}", db_path_str);
    
    // Remove the file if it exists
    if db_path.exists() {
        std::fs::remove_file(&db_path).unwrap();
    }
    // Ensure the file exists before connecting
    std::fs::File::create(&db_path)?;
    
    // Create a test database using SQLx
    let conn_str = format!("sqlite://{}", db_path_str);
    println!("Connection string: {}", conn_str);
    
    let mut conn = SqliteConnection::connect(&conn_str).await?;
    
    sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
        .execute(&mut conn)
        .await?;
    
    sqlx::query("INSERT INTO test (name) VALUES (?)")
        .bind("Test Name")
        .execute(&mut conn)
        .await?;
    
    // Close the connection
    drop(conn);

    // Create the adapter
    let adapter = SqliteAdapter::new(db_path_str).await?;

    // Test getting all tables
    let tables = adapter.get_all_tables().await?;
    assert_eq!(tables, vec!["test".to_string()]);

    // Test getting table columns
    let columns = adapter.get_table_columns("test").await?;
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0].0, "id");
    assert_eq!(columns[0].1, "INTEGER");
    assert!(columns[0].2); // is primary key
    assert_eq!(columns[1].0, "name");
    assert_eq!(columns[1].1, "TEXT");
    assert!(!columns[1].2); // not primary key

    // Test getting primary key
    let primary_key = adapter.get_primary_key("test").await?;
    assert_eq!(primary_key, "id");

    // Test fetching all records
    let records = adapter.fetch_all_records("test").await?;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].get("id").unwrap().as_i64().unwrap(), 1);
    assert_eq!(records[0].get("name").unwrap().as_str().unwrap(), "Test Name");

    // Clean up the file
    std::fs::remove_file(&db_path).unwrap_or_default();

    Ok(())
}

#[tokio::test]
async fn test_mock_sqlite_adapter() -> Result<()> {
    // Create a simple mock adapter within the test
    struct MockAdapter;
    
    #[async_trait]
    impl DatabaseAdapter for MockAdapter {
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
    
    let adapter = MockAdapter;
    
    // Test the adapter methods
    let tables = adapter.get_all_tables().await?;
    assert_eq!(tables, vec!["test".to_string()]);
    
    let columns = adapter.get_table_columns("test").await?;
    assert_eq!(columns.len(), 1);
    assert_eq!(columns[0].0, "id");
    assert_eq!(columns[0].1, "INTEGER");
    assert!(columns[0].2); // primary key
    
    let primary_key = adapter.get_primary_key("test").await?;
    assert_eq!(primary_key, "id");
    
    // Test that fetching records returns empty array
    let records = adapter.fetch_all_records("test").await?;
    assert!(records.is_empty());
    
    Ok(())
}

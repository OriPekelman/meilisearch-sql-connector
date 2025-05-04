#[path = "utils.rs"]
mod utils;
use utils::start_meilisearch;
use meilisearch_sdk::client::Client;
use meilisearch_sdk::search::SearchResults;
use meilisearch_sql_connector::config::Config;
use meilisearch_sql_connector::connector::Connector;
use sqlx::{Connection, SqliteConnection, Row};
use std::time::Duration;
use tokio::time::sleep;
use std::os::unix::fs::PermissionsExt;

#[tokio::test]
async fn test_docs_example() -> Result<(), Box<dyn std::error::Error>> {
    // Use a persistent tmp directory in the project root
    let project_tmp = std::env::current_dir()?.join("tmp");
    std::fs::create_dir_all(&project_tmp)?;
    let db_path = project_tmp.join("test.db");
    let db_path_str = db_path.to_str().unwrap().to_string();

    // Remove the file if it exists
    if db_path.exists() {
        println!("Removing file {}", db_path_str);
        std::fs::remove_file(&db_path)?;
    }

    // Ensure the file exists before connecting
    std::fs::File::create(&db_path)?;

    // Setup test database first
    let conn_str = format!("sqlite://{}", db_path_str);
    println!("Connection string {}", conn_str);
    let mut conn = SqliteConnection::connect(&conn_str).await?;
    // Create a table with an integer primary key
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS test_table_int (
            id INTEGER PRIMARY KEY,
            field1 TEXT,
            field2 TEXT
        )",
    )
    .execute(&mut conn)
    .await?;
    // Create a table with a string primary key (e.g., UUID)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS test_table_str (
            id TEXT PRIMARY KEY,
            field1 TEXT,
            field2 TEXT
        )",
    )
    .execute(&mut conn)
    .await?;
    // Insert test data with integer primary key
    sqlx::query("INSERT INTO test_table_int (id, field1, field2) VALUES (1, 'test1', 'value1')")
        .execute(&mut conn)
        .await?;
    sqlx::query("INSERT INTO test_table_int (id, field1, field2) VALUES (2, 'test2', 'value2')")
        .execute(&mut conn)
        .await?;
    // Insert test data with string primary key
    sqlx::query("INSERT INTO test_table_str (id, field1, field2) VALUES ('uuid-1', 'test1', 'value1')")
        .execute(&mut conn)
        .await?;
    sqlx::query("INSERT INTO test_table_str (id, field1, field2) VALUES ('uuid-2', 'test2', 'value2')")
        .execute(&mut conn)
        .await?;
    // Print the documents in test_table_int before running the connector
    let rows = sqlx::query("SELECT * FROM test_table_int").fetch_all(&mut conn).await?;
    for row in &rows {
        let id: i64 = row.try_get("id").unwrap_or(-1);
        let field1: String = row.try_get("field1").unwrap_or_default();
        let field2: String = row.try_get("field2").unwrap_or_default();
        println!("[test] DB row: id={}, field1={}, field2={}", id, field1, field2);
    }
    drop(conn);

    // Start Meilisearch if not already running
    let meilisearch = start_meilisearch().await?;

    // Initialize Meilisearch client
    let client = Client::new("http://localhost:7701", None::<String>)?;

    // Delete indices if they exist
    for index_name in &["test_index_int", "test_index_str"] {
        if let Ok(index) = client.get_index(index_name).await {
            let task = index.delete().await?;
            task.wait_for_completion(&client, None, None).await?;
        }
    }

    // Load and modify config to use the persistent database
    let config_path = std::env::current_dir()?.join("tests/fixtures/config.toml");
    println!("Loading config from: {}", config_path.display());
    let mut config = Config::from_file(config_path.to_str().unwrap())?;
    println!("Config loaded successfully");

    // Update the database path in the config
    config.database.connection_string = db_path_str.clone();
    eprintln!("[test] DB connection string: {}", config.database.connection_string);
    eprintln!("[test] File exists at {}: {}", db_path_str, std::path::Path::new(&db_path_str).exists());
    if !std::path::Path::new(&db_path_str).exists() {
        panic!("[test] DB connection string: {} | File exists at {}: {}", config.database.connection_string, db_path_str, std::path::Path::new(&db_path_str).exists());
    }

    let metadata = std::fs::metadata(&db_path_str).unwrap();
    eprintln!("[test] File permissions for {}: {:o}", db_path_str, metadata.permissions().mode());
    match std::fs::File::open(&db_path_str) {
        Ok(_) => eprintln!("[test] Successfully opened file with std::fs::File::open: {}", db_path_str),
        Err(e) => panic!("[test] Failed to open file with std::fs::File::open: {} | Error: {}", db_path_str, e),
    }

    // Print the database path used for SQLx
    println!("[test] SQLx DB path: {}", db_path_str);

    let connector = Connector::new(config).await?;
    println!("Connector created successfully");
    connector.sync_once().await?;
    println!("Initial sync completed successfully");

    // Check Meilisearch task queue for indexing status
    let index = client.get_index("test_index_int").await?;
    let index_tasks = index.get_tasks().await?;
    println!("[test] Meilisearch tasks for index 'test_index_int' after sync: {:#?}", index_tasks);

    // Wait for initial sync and indexing
    sleep(Duration::from_secs(5)).await;

    // Test integer primary key table
    let index = client.get_index("test_index_int").await?;
    let search_result: SearchResults<serde_json::Value> =
        index.search().with_query("test1").execute().await?;

    println!("Search result structure: {:#?}", search_result);
    if search_result.hits.is_empty() {
        println!("[test] No hits found in Meilisearch for query 'test1'.");
        // Optionally, fetch all documents in the index for debugging
        let all_docs: SearchResults<serde_json::Value> = index.search().execute().await?;
        println!("[test] All docs in index: {:#?}", all_docs);
    }
    sleep(Duration::from_secs(1)).await;
    // Defensive: only access hits[0] if not empty
    assert!(!search_result.hits.is_empty(), "No hits found in Meilisearch for query 'test1'");
    println!("First hit: {:#?}", search_result.hits[0]);
    println!("First hit result: {:#?}", search_result.hits[0].result);
    println!("First hit result id: {:#?}", search_result.hits[0].result["id"]);

    assert_eq!(search_result.hits[0].result["field1"], "test1");

    // Cleanup
    if let Some(mut meilisearch) = meilisearch {
        meilisearch.kill()?;
        // Only clean up after Meilisearch is killed
        if std::path::Path::new(&db_path_str).exists() {
            std::fs::remove_file(&db_path_str)?;
        }
    }

    Ok(())
}

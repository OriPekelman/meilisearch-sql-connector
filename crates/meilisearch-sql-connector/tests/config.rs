use meilisearch_sql_connector::config::Config;
use std::fs;

#[test]
fn test_config_load() {
    let config_str = r#"
        [meilisearch]
        host = "http://localhost:7701"
        api_key = "test_key"

        [database]
        type = "sqlite"
        connection_string = "test.db"
        poll_interval_seconds = 10

        [[database.tables]]
        name = "test_table"
        primary_key = "id"
        index_name = "test_index"
        fields_to_index = ["field1", "field2"]
        watch_for_changes = true
    "#;

    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    fs::write(&config_path, config_str).unwrap();

    let config = Config::from_file(&config_path).unwrap();
    assert_eq!(config.meilisearch.host, "http://localhost:7701");
    assert_eq!(config.meilisearch.api_key, Some("test_key".to_string()));
    assert_eq!(config.database.type_, "sqlite");
    assert_eq!(config.database.connection_string, "test.db");
    assert_eq!(config.database.poll_interval_seconds, Some(10));
    assert_eq!(config.database.tables.len(), 1);

    let table = &config.database.tables[0];
    assert_eq!(table.name, "test_table");
    assert_eq!(table.primary_key, "id");
    assert_eq!(table.index_name, Some("test_index".to_string()));
    assert_eq!(table.fields_to_index, vec!["field1", "field2"]);
    assert!(table.watch_for_changes);
}

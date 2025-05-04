use meilisearch_sql_connector::config::Config;
use meilisearch_sql_connector::error::ConnectorError;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use std::process::Child;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;
use meilisearch_sdk::client::Client;

#[allow(dead_code)]
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub db_path: PathBuf,
    pub meili_url: String,
}

impl TestEnvironment {
    #[allow(dead_code)]
    pub async fn new() -> Result<Self, ConnectorError> {
        let temp_dir = TempDir::new().map_err(|e| ConnectorError::Config(e.to_string()))?;
        let db_path = temp_dir.path().join("test.db");
        let meili_url = "http://localhost:7701".to_string();

        Ok(Self { temp_dir, db_path, meili_url })
    }

    #[allow(dead_code)]
    pub fn config(&self) -> Config {
        let config_str = format!(
            r#"
            [meilisearch]
            host = "{}"
            api_key = "masterKey"

            [database]
            type = "sqlite"
            connection_string = "{}"
            poll_interval_seconds = 1

            [[database.tables]]
            name = "test_table"
            primary_key = "id"
            index_name = "test_index"
            fields_to_index = ["field1", "field2"]
            watch_for_changes = true
            "#,
            self.meili_url,
            self.db_path.display()
        );

        let config_path = self.temp_dir.path().join("config.toml");
        fs::write(&config_path, config_str).unwrap();
        Config::from_file(&config_path).unwrap()
    }
}

#[allow(dead_code)]
pub async fn start_meilisearch() -> Result<Option<Child>, Box<dyn std::error::Error>> {
    // Always attempt to kill existing instances first
    println!("Attempting to kill existing meilisearch processes...");
    let kill_output = Command::new("killall").arg("meilisearch").output();
    match kill_output {
        Ok(output) => println!("killall status: {}, stdout: {}, stderr: {}", 
                                output.status, 
                                String::from_utf8_lossy(&output.stdout),
                                String::from_utf8_lossy(&output.stderr)),
        Err(e) => println!("killall command failed: {}", e),
    }
    // Brief pause to allow process termination
    sleep(Duration::from_millis(500)).await;

    // Now, try to connect. If it succeeds, something is wrong.
    let client = Client::new("http://localhost:7701", None::<String>)?;
    match client.health().await {
        Ok(_) => {
            println!("Error: Meilisearch is unexpectedly running on port 7701 after kill attempt.");
            // Consider this an error state, as we expect no instance now.
            // Optionally, try killing again more forcefully or return an error.
            // For now, we'll proceed assuming it might be a zombie process we can't kill
            // and hope spawning a new one works or errors out.
             Err("Meilisearch running unexpectedly after kill attempt".into())
        }
        Err(_) => {
            // Health check failed, which is expected. Proceed to start.
            println!("Starting Meilisearch...");
            let current_dir = std::env::current_dir()?;
            let data_path = current_dir.as_path().join("tmp/data.ms");
            let dump_path = current_dir.as_path().join("tmp/dumps");
            // Clean up directories before starting
            if data_path.exists() {
                std::fs::remove_dir_all(&data_path)?;
            }
            if dump_path.exists() {
                std::fs::remove_dir_all(&dump_path)?;
            }
            std::fs::create_dir_all(&data_path)?;
            std::fs::create_dir_all(&dump_path)?;
            let meilisearch = Command::new("meilisearch")
                .arg("--db-path")
                .arg(&data_path)
                .arg("--dump-dir")
                .arg(&dump_path)
                .arg("--http-addr")
                .arg("localhost:7701")
                .spawn()?;
            // Wait for the *new* instance to become healthy
            let mut retries = 0;
            while retries < 20 { // Increased retries slightly
                sleep(Duration::from_secs(1)).await;
                match client.health().await {
                    Ok(_) => {
                        println!("New Meilisearch instance started successfully.");
                        // No extra sleep needed here now, handled by wait_for_task later
                        return Ok(Some(meilisearch));
                    }
                    Err(_) => {
                        retries += 1;
                        println!("Waiting for Meilisearch to start... (attempt {})", retries);
                    }
                }
            }
            Err("Failed to start Meilisearch after retries".into())
        }
    }
}

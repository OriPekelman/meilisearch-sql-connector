use crate::config::{Config, DatabaseConfig, MeilisearchConfig, TableConfig};
use crate::database::sqlite::SqliteAdapter;
use crate::error::{Error, Result};
use serde_json::json;
use std::path::Path;

pub struct ConfigGenerator {
    adapter: SqliteAdapter,
}

impl ConfigGenerator {
    pub fn new(connection_string: &str) -> Result<Self> {
        Ok(Self {
            adapter: SqliteAdapter::new(connection_string)?,
        })
    }

    pub async fn generate_config(&self, meilisearch_host: &str, meilisearch_api_key: Option<&str>) -> Result<Config> {
        let tables = self.adapter.fetch_all_tables().await?;
        let mut table_configs = Vec::new();

        for table in tables {
            let schema = self.adapter.get_table_schema(&table).await?;
            let fields_to_index = schema.columns.iter()
                .map(|col| col.name.clone())
                .collect();

            table_configs.push(TableConfig {
                name: table.clone(),
                primary_key: schema.primary_key,
                index_name: table,
                fields_to_index,
                watch_for_changes: true,
                index_config: None,
            });
        }

        Ok(Config {
            meilisearch: MeilisearchConfig {
                host: meilisearch_host.to_string(),
                api_key: meilisearch_api_key.map(String::from),
            },
            database: DatabaseConfig {
                db_type: "sqlite".to_string(),
                connection_string: self.adapter.connection_string().to_string(),
                poll_interval_seconds: 60,
                tables: table_configs,
            },
        })
    }

    pub async fn save_config(&self, path: &Path, config: &Config) -> Result<()> {
        let json = json!(config);
        std::fs::write(path, serde_json::to_string_pretty(&json)?)
            .map_err(|e| Error::ConfigError(format!("Failed to write config file: {}", e)))?;
        Ok(())
    }
} 
use crate::database::DatabaseAdapter;
use crate::error::{ConnectorError, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::Path;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub meilisearch: MeilisearchConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(rename = "type")]
    pub type_: String,
    pub connection_string: String,
    pub poll_interval_seconds: Option<u64>,
    pub tables: Vec<TableConfig>,
    // Performance tuning parameters
    #[serde(default = "default_connection_pool_size")]
    pub connection_pool_size: u32,
    #[serde(default = "default_max_concurrent_batches")]
    pub max_concurrent_batches: usize,
    #[serde(default = "default_document_batch_size")]
    pub document_batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeilisearchConfig {
    pub host: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableConfig {
    pub name: String,
    pub primary_key: String,
    pub index_name: Option<String>,
    pub fields_to_index: Vec<String>,
    pub watch_for_changes: bool,
    pub searchable_attributes: Option<Vec<String>>,
    pub ranking_rules: Option<Vec<String>>,
    pub typo_tolerance: Option<TypoToleranceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypoToleranceConfig {
    pub enabled: bool,
}

// Default values for performance configuration
fn default_connection_pool_size() -> u32 {
    5
}

fn default_max_concurrent_batches() -> usize {
    5
}

fn default_document_batch_size() -> usize {
    100
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents =
            std::fs::read_to_string(path).map_err(|e| ConnectorError::Config(e.to_string()))?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub async fn generate_from_database_url(
        database_url: &str,
        meilisearch_host: &str,
        poll_interval_seconds: u64,
    ) -> Result<Self> {
        let url = Url::parse(database_url)
            .map_err(|e| ConnectorError::Config(format!("Invalid database URL: {}", e)))?;
        
        let db_type = url.scheme().to_string();
        let connection_string = match db_type.as_str() {
            "sqlite" => url.path().to_string(),
            _ => return Err(ConnectorError::UnsupportedDatabaseType(db_type)),
        };

        let adapter = match db_type.as_str() {
            "sqlite" => crate::database::sqlite::SqliteAdapter::new(&connection_string).await?,
            _ => return Err(ConnectorError::UnsupportedDatabaseType(db_type)),
        };

        let tables = adapter.get_all_tables().await?;
        let mut table_configs = Vec::new();

        for table in tables {
            let table = table.to_string();
            let columns = adapter.get_table_columns(&table).await?;
            
            // Try to get primary key, but don't error if not found - just skip the table
            match adapter.get_primary_key(&table).await {
                Ok(primary_key) => {
                    // Add the table to our configuration
                    table_configs.push(TableConfig {
                        name: table,
                        primary_key,
                        index_name: None,
                        fields_to_index: columns.iter().map(|(name, _, _)| name.clone()).collect(),
                        watch_for_changes: true,
                        searchable_attributes: None,
                        ranking_rules: None,
                        typo_tolerance: Some(TypoToleranceConfig { enabled: true }),
                    });
                },
                Err(ConnectorError::NoPrimaryKey(_)) => {
                    // Table has no primary key, print a warning and skip it
                    eprintln!("{} Skipping table '{}' as it has no primary key", 
                              "Warning:".yellow().bold(), 
                              table.yellow());
                },
                Err(e) => return Err(e), // Pass through other errors
            }
        }

        // Make sure we have at least one table configured
        if table_configs.is_empty() {
            return Err(ConnectorError::Config("No tables with primary keys found in database".to_string()));
        }

        Ok(Self {
            database: DatabaseConfig {
                type_: db_type,
                connection_string,
                poll_interval_seconds: Some(poll_interval_seconds),
                tables: table_configs,
                connection_pool_size: default_connection_pool_size(),
                max_concurrent_batches: default_max_concurrent_batches(),
                document_batch_size: default_document_batch_size(),
            },
            meilisearch: MeilisearchConfig { host: meilisearch_host.to_string(), api_key: None },
        })
    }

    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string(self)?)
    }
}

impl TableConfig {
    // Remove to_settings method
}

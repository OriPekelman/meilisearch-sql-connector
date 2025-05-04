use std::error::Error;
use std::fmt;
use std::io;

pub type Result<T> = std::result::Result<T, ConnectorError>;

#[derive(Debug)]
pub enum ConnectorError {
    Database(String),
    Meilisearch(String),
    Config(String),
    TomlSerialization(String),
    NoPrimaryKey(String),
    UnsupportedDatabaseType(String),
    Io(String),
}

impl fmt::Display for ConnectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectorError::Database(msg) => write!(f, "Database error: {}", msg),
            ConnectorError::Meilisearch(msg) => write!(f, "Meilisearch error: {}", msg),
            ConnectorError::Config(msg) => write!(f, "Config error: {}", msg),
            ConnectorError::TomlSerialization(msg) => write!(f, "TOML serialization error: {}", msg),
            ConnectorError::NoPrimaryKey(table) => write!(f, "No primary key found for table: {}", table),
            ConnectorError::UnsupportedDatabaseType(db_type) => write!(f, "Unsupported database type: {}", db_type),
            ConnectorError::Io(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl Error for ConnectorError {}

impl From<sqlx::Error> for ConnectorError {
    fn from(err: sqlx::Error) -> Self {
        ConnectorError::Database(err.to_string())
    }
}

impl From<meilisearch_sdk::errors::Error> for ConnectorError {
    fn from(err: meilisearch_sdk::errors::Error) -> Self {
        ConnectorError::Meilisearch(err.to_string())
    }
}

impl From<toml::de::Error> for ConnectorError {
    fn from(err: toml::de::Error) -> Self {
        ConnectorError::Config(err.to_string())
    }
}

impl From<toml::ser::Error> for ConnectorError {
    fn from(err: toml::ser::Error) -> Self {
        ConnectorError::TomlSerialization(err.to_string())
    }
}

impl From<io::Error> for ConnectorError {
    fn from(err: io::Error) -> Self {
        ConnectorError::Io(err.to_string())
    }
}

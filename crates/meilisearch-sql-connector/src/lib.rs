//! The main entry point for the Meilisearch SQL Connector.
//!
//! This crate provides a connector that synchronizes SQL databases with Meilisearch,
//! supporting automatic schema detection, change tracking, and zero-configuration setup.
//!
//! # Examples
//!
//! ```no_run
//! use meilisearch_sql_connector::{Config, Connector};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::load("config.toml")?;
//!     let connector = Connector::new(config)?;
//!     connector.start().await?;
//!     Ok(())
//! }
//! ```

pub mod cli;
pub mod config;
pub mod connector;
pub mod database;
pub mod error;
pub mod logging;
pub mod meilisearch;

#[cfg(feature = "test")]
pub mod common;

pub use config::Config;
pub use connector::Connector;
pub use error::ConnectorError;

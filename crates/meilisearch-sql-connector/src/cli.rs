use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the connector with the specified configuration
    Run {
        /// Path to the configuration file
        #[arg(short, long)]
        config: PathBuf,
    },
    /// Generate a configuration file from an existing database
    Generate {
        /// Database URL (e.g. sqlite://path/to/database.db)
        #[arg(short, long)]
        database_url: String,
        /// Meilisearch host URL
        #[arg(short, long)]
        meilisearch_host: String,
        /// Meilisearch API key (optional)
        #[arg(short = 'k', long)]
        meilisearch_key: Option<String>,
        /// Output path for the configuration file
        #[arg(short, long)]
        output: PathBuf,
        /// Polling interval in seconds
        #[arg(short, long, default_value = "60")]
        poll_interval: u64,
    },
    /// Validate a configuration file
    Validate {
        /// Path to the configuration file
        #[arg(short, long)]
        config: PathBuf,
    },
}

pub fn print_banner() {
    println!("\n{}", "Meilisearch SQL Connector".bold());
    println!("{}", "A connector that syncs your SQL database with Meilisearch".italic());
    println!("{}", "Usage: meilisearch-sql-connector run --config config.toml".bold());
    println!("{}", "Usage: meilisearch-sql-connector generate --database-url sqlite://path/to/database.db --meilisearch-host http://localhost:7701 [--meilisearch-key YOUR_KEY] --output config.toml --poll-interval 60".bold());
    println!("{}", "Usage: meilisearch-sql-connector validate --config config.toml".bold());
    println!();
}

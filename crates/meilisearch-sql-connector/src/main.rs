mod cli;
mod config;
mod connector;
mod database;
mod error;
mod meilisearch;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use colored::Colorize;
use std::fs;
use std::sync::Arc;
use tokio::signal;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    // Use RUST_LOG environment variable or default to info
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();

    let cli = Cli::parse();

    if cli.command.is_none() {
        cli::print_banner();
        // Optional: print default help too
        // use clap::CommandFactory;
        // Cli::command().print_help()?;
    } else if let Some(command) = cli.command {
        match command {
            Commands::Run { config } => {
                let config = config::Config::from_file(&config)?;
                let connector = Arc::new(connector::Connector::new(config).await?);
                
                // Clone for signal handler
                let connector_for_signal = connector.clone();
                
                // Setup Ctrl+C handler
                tokio::spawn(async move {
                    match signal::ctrl_c().await {
                        Ok(()) => {
                            println!("Ctrl+C received, shutting down...");
                            let _ = connector_for_signal.stop().await;
                        },
                        Err(err) => {
                            eprintln!("Error setting up Ctrl+C handler: {}", err);
                        },
                    }
                });
                
                connector.start().await?;
            }
            Commands::Generate { database_url, meilisearch_host, meilisearch_key, output, poll_interval } => {
                println!("{}", "Generating configuration...".green());
                let mut config = config::Config::generate_from_database_url(
                    &database_url,
                    &meilisearch_host,
                    poll_interval,
                ).await?;
                
                // Set the API key if provided
                if let Some(key) = meilisearch_key {
                    config.meilisearch.api_key = Some(key);
                }
                
                fs::write(&output, config.to_toml()?)?;
                println!(
                    "{} Configuration generated successfully at {}",
                    "✓".green(),
                    output.display()
                );
            }
            Commands::Validate { config } => {
                println!("{}", "Validating configuration...".green());
                let _config = config::Config::from_file(&config)?;
                println!("{} Configuration is valid", "✓".green());
            }
        }
    }

    Ok(())
}

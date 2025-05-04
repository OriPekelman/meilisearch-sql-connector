# Meilisearch SQL Connector Architecture

This document outlines the architecture and code organization of the Meilisearch SQL Connector project.

## Project Structure

```
meilisearch-sql-connector/
├── Cargo.toml
├── README.md
├── ARCHITECTURE.md
├── TODO.md
└── crates/
    └── meilisearch-sql-connector/
        ├── src/
        │   ├── main.rs
        │   ├── cli.rs
        │   ├── lib.rs
        │   ├── connector.rs
        │   ├── error.rs
        │   ├── logging.rs
        │   ├── common.rs  # (If applicable, or remove if test-only)
        │   ├── config/    # Directory
        │   │   └── ...    # (mod.rs, etc.)
        │   ├── database/  # Directory
        │   │   ├── mod.rs
        │   │   ├── sqlite.rs
        │   │   ├── postgres.rs # Stub
        │   │   └── mysql.rs    # Stub
        │   └── meilisearch/ # Directory
        │       ├── mod.rs
        │       └── client.rs
        └── tests/
            ├── mod.rs
            ├── config.rs
            ├── connector.rs
            ├── docs.rs
            ├── error.rs
            ├── integration.rs
            ├── sqlite.rs
            ├── test_runner.rs # (Test helper)
            └── utils.rs       # (Test helper)
```

## Core Components

### CLI (`src/cli.rs`)
- Handles command-line interface using `clap`
- Supports three main commands:
  - `run`: Execute the connector with a configuration file
  - `generate`: Create configuration from an existing database
  - `validate`: Validate a configuration file

### Configuration (`src/config/`)
- Manages configuration parsing and validation
- Defines configuration structures for:
  - Meilisearch settings
  - Database connection details
  - Table-specific indexing rules
  - Search settings (ranking rules, typo tolerance, etc.)

### Connector (`src/connector.rs`)
- Core synchronization logic
- Manages the connection between database and Meilisearch
- Handles change detection and document synchronization
- Implements error recovery and retry mechanisms

### Database Adapter (`src/database/`)
- Abstract interface for database operations (`DatabaseAdapter` trait)
- SQLite implementation (`sqlite.rs`)
- Handles:
  - Table schema detection
  - Primary key identification
  - Record fetching and change detection
  - Type conversion between SQL and Meilisearch

### Meilisearch Client (`src/meilisearch/`)
- Interface for Meilisearch operations
- Handles:
  - Index creation and configuration
  - Document addition/update/deletion
  - Search settings management
  - Error handling and retries

### Error Handling (`src/error.rs`)
- Centralized error types and handling
- Categorizes errors (transient vs. permanent)
- Provides detailed error context for debugging

## Testing Structure

### Unit Tests
- Located within the corresponding `src/` files or dedicated test files in `tests/`.
- Mocks are often defined within the test modules requiring them.

### Integration Tests (`tests/integration.rs`, `tests/docs.rs`)
- `integration.rs`: End-to-end tests with real Meilisearch instance
- Tests real-world scenarios:
  - Initial synchronization
  - Document updates
  - Schema changes
  - Error recovery

### Test Utilities (`tests/utils.rs`, `tests/test_runner.rs`)
- `utils.rs`: Contains shared helpers like Meilisearch process starting.
- `test_runner.rs`: Contains helpers, currently unused `TestRunner` struct.
- `TestEnvironment` in `utils.rs` is currently unused.

## Key Design Principles

1. **Modularity**: Clear separation of concerns between components
2. **Extensibility**: Easy to add new database adapters
3. **Robustness**: Comprehensive error handling and recovery
4. **Testability**: Extensive test coverage with mock implementations
5. **Configuration First**: Flexible configuration system
6. **Zero Configuration**: Automatic schema detection and setup
7. **Performance Tuning**: Configurable parallelism and batch processing

## Performance Architecture

The connector implements several strategies to optimize performance:

1. **Connection Pooling**: Database connections are managed via a connection pool, with configurable size
2. **Parallel Document Processing**: Documents are processed in batches with configurable concurrency
3. **Batched Operations**: Documents are sent to Meilisearch in controlled batch sizes
4. **Incremental Updates**: Only changed documents are processed, reducing resource usage
5. **Optimized Document Comparison**: Using HashMaps for efficient document change detection
6. **Throttled Operations**: Small delays between batches prevent overwhelming the Meilisearch server

The configuration allows fine-tuning these parameters:

- `connection_pool_size`: Number of database connections (default: 5)
- `max_concurrent_batches`: Maximum parallel batch operations (default: 5)
- `document_batch_size`: Number of documents per batch (default: 100)

## Future Architecture Considerations

1. **PostgreSQL Support**: Planned implementation of PostgreSQL adapter
2. **Custom SQL Queries**: Support for custom SQL queries in configuration
3. **View Support**: Database view synchronization
4. **Monitoring**: Prometheus metrics and health checks
5. **Web UI**: Configuration management interface 
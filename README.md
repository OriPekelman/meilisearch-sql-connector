# Meilisearch SQL Connector

A SQL database connector for Meilisearch that supports automatic schema detection, change tracking, and zero-configuration setup. The connector handles various primary key types (integers, UUIDs, strings) and automatically maps database schemas to Meilisearch indices.

## Current status

Well, this is just a fun protoype for the time being. Really **DO NOT TRY AND USE THIS**.

## Features

- **Zero Configuration**: Automatically detect and index all tables in your SQLite database
- **Flexible Primary Keys**: Support for integer, UUID, and string primary keys
- **Schema Change Detection**: Automatically detect and handle schema changes
- **Robust Error Handling**: Comprehensive error handling and recovery mechanisms
- **Configuration Generation**: Generate configuration files from existing databases
- **Real-time Sync**: Polling-based change detection with configurable intervals
- **Schema Validation**: Validate database schemas against configuration
- **Index Management**: Automatic index creation and configuration
- **Type Handling**: Automatic type detection and conversion between SQLite and Meilisearch

## Installation

```bash
cargo install meilisearch-sql-connector
```

## Usage

The connector provides three main commands:

### Run the Connector

Run the connector with a configuration file:

```bash
meilisearch-sql-connector run --config config.toml
```

### Generate Configuration

Generate a configuration file from an existing database:

```bash
meilisearch-sql-connector generate \
  --database-url sqlite:///path/to/database.db \
  --meilisearch-host http://localhost:7700 \
  --meilisearch-key YOUR_MEILISEARCH_API_KEY \
  --output config.toml \
  --poll-interval 60
```

The `--meilisearch-key` parameter is optional and can be omitted if you're using Meilisearch without API key authentication.

### Validate Configuration

Validate a configuration file:

```bash
meilisearch-sql-connector validate --config config.toml
```

## Configuration

The connector supports both automatic configuration generation and manual configuration. Here's an example configuration file:

```toml
[meilisearch]
host = "http://localhost:7700"
api_key = "optional-api-key"

[database]
type = "sqlite"
connection_string = "path/to/database.db"
poll_interval_seconds = 60
# Performance tuning parameters (optional)
connection_pool_size = 10               # Number of database connections in the pool
max_concurrent_batches = 8              # Maximum number of concurrent batch operations
document_batch_size = 200               # Number of documents per batch

[[database.tables]]
name = "users"
primary_key = "id"  # Can be integer or string (UUID)
index_name = "users"
fields_to_index = ["id", "name", "email"]  # Include primary key in fields_to_index
watch_for_changes = true
searchable_attributes = ["name", "email"]
ranking_rules = ["exactness", "words", "typo", "proximity", "attribute", "sort"]
typo_tolerance = { enabled = false }
```

### Performance Tuning

The connector includes several configuration options for performance tuning:

1. **`connection_pool_size`**: Controls the number of database connections in the pool (default: 5)
2. **`max_concurrent_batches`**: Limits the number of concurrent batch operations when syncing documents (default: 5)
3. **`document_batch_size`**: Sets the number of documents processed in each batch (default: 100)

For large databases, you may want to increase these values to improve throughput. However, setting them too high can overload Meilisearch or your database. We recommend testing different configurations to find the optimal balance for your specific setup.

## Primary Key Handling

The connector automatically handles different types of primary keys:

1. **Integer Primary Keys**: Commonly used auto-incrementing IDs
2. **String Primary Keys**: UUIDs or other string-based identifiers
3. **Type Preservation**: Primary key types are preserved when syncing to Meilisearch

## Schema Change Handling

The connector automatically detects and handles schema changes:

1. **Column Additions**: New columns are automatically detected and added to the index
2. **Column Removals**: Removed columns are automatically removed from the index
3. **Primary Key Changes**: Index is recreated with the new primary key
4. **Type Changes**: Schema version changes trigger a full reindex

## Error Handling

The connector includes comprehensive error handling:

- **Connection Errors**: Automatic retry with exponential backoff
- **Schema Validation**: Validates configuration against database schema
- **Index Management**: Handles index creation and updates gracefully
- **Change Detection**: Robust polling mechanism with error recovery

## Development

### Building from Source

```bash
git clone https://github.com/yourusername/meilisearch-sql-connector.git
cd meilisearch-sql-connector
cargo build --release
```

### Running Tests

```bash
cargo test
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
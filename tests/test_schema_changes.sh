#!/bin/bash

# Create test database
rm -f tmp/test.db
sqlite3 tmp/test.db < tests/setup_test_db.sql

# Generate initial configuration
cargo run -- generate \
    --database tmp/test.db \
    --meilisearch-host http://localhost:7701 \
    --output config.json

# Start Meilisearch in the background
meilisearch --db-path tmp/data.ms --dump-dir tmp/dumps --http-addr "localhost:7701" &
MEILISEARCH_PID=$!

# Wait for Meilisearch to start
sleep 5

# Run the connector in the background
cargo run -- run --config config.json &
CONNECTOR_PID=$!

# Wait for initial sync
sleep 10

# Test 1: Add a new column
echo "Testing: Adding a new column"
sqlite3 tmp/test.db "ALTER TABLE users ADD COLUMN phone TEXT;"
sleep 5

# Test 2: Add a new table
echo "Testing: Adding a new table"
sqlite3 tmp/test.db "CREATE TABLE orders (
    id INTEGER PRIMARY KEY,
    user_id INTEGER,
    product_id INTEGER,
    quantity INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (product_id) REFERENCES products(id)
);"
sleep 5

# Test 3: Modify column type
echo "Testing: Modifying column type"
sqlite3 tmp/test.db "ALTER TABLE products ADD COLUMN price_new DECIMAL(10,2);"
sqlite3 tmp/test.db "UPDATE products SET price_new = price;"
sqlite3 tmp/test.db "ALTER TABLE products DROP COLUMN price;"
sqlite3 tmp/test.db "ALTER TABLE products RENAME COLUMN price_new TO price;"
sleep 5

# Test 4: Change primary key
echo "Testing: Changing primary key"
sqlite3 tmp/test.db "ALTER TABLE users ADD COLUMN uuid TEXT UNIQUE;"
sqlite3 tmp/test.db "UPDATE users SET uuid = 'user-' || id;"
sleep 5

# Cleanup
kill $CONNECTOR_PID
kill $MEILISEARCH_PID
rm -f tmp/test.db config.json 
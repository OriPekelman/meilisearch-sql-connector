#!/bin/bash

# Kill any existing Meilisearch processes
killall meilisearch || true

# Create a temporary directory for test files
TEMP_DIR="tmp"
echo "Using temporary directory: $TEMP_DIR"

# Set up environment variable for the test database path
export TEST_DB_PATH="$TEMP_DIR/test.db"

MEILI_DATA_DIR="$TEMP_DIR"

# Function to clean up
cleanup() {
    echo "Cleaning up..."
    killall meilisearch || true
    rm -rf "$TEMP_DIR"
}

# Set up trap to clean up on script exit
trap cleanup EXIT

# Start Meilisearch
echo "Starting Meilisearch..."
meilisearch --db-path "$TEMP_DIR/data.ms" --dump-dir "$TEMP_DIR/dumps" --http-addr "localhost:7701" &
MEILI_PID=$!

# Wait for Meilisearch to start
sleep 2

# Run the tests
echo "Running tests..."
cargo test -- --nocapture

# Store the exit code
EXIT_CODE=$?

# Clean up
cleanup

# Exit with the test's exit code
exit $EXIT_CODE 
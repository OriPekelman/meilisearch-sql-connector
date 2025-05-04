#[path = "utils.rs"]
mod utils;
use std::process::Child;

#[allow(dead_code)]
pub struct TestRunner {
    meilisearch: Option<Child>,
}

impl TestRunner {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            meilisearch: None,
        }
    }

    #[allow(dead_code)]
    pub fn get_db_path(&self) -> String {
        let current_dir = std::env::current_dir().unwrap();
        current_dir.as_path().join("tmp/test.db").to_str().unwrap().to_string()
    }

    pub fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Kill Meilisearch if it's running
        if let Some(mut meilisearch) = self.meilisearch.take() {
            meilisearch.kill()?;
        }

        // Temp directory will be automatically cleaned up when dropped
        Ok(())
    }
}

impl Drop for TestRunner {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
} 
use async_trait::async_trait;

/// Storage backend abstraction for Rad.
/// Allows pluggable storage implementations (filesystem, S3, etc.)
#[async_trait]
pub trait RadStorageBackend: Send + Sync {
    /// Store data at the given key
    async fn put(&self, key: &str, data: &str) -> Result<(), String>;

    /// Retrieve data from the given key
    /// Returns None if key does not exist
    async fn get(&self, key: &str) -> Result<Option<String>, String>;

    /// List all keys with the given prefix
    async fn list(&self, prefix: &str) -> Result<Vec<String>, String>;

    /// Delete the object at the given key
    async fn delete(&self, key: &str) -> Result<(), String>;
}

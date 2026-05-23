use async_trait::async_trait;
use std::fs;
use std::path::{Path, PathBuf};
use super::backend::RadStorageBackend;

/// Filesystem-based storage backend.
/// Stores data in a directory structure (e.g., .rad/)
pub struct FileSystemBackend {
    rad_dir: PathBuf,
}

impl FileSystemBackend {
    pub fn new(rad_dir: PathBuf) -> Self {
        Self { rad_dir }
    }
}

#[async_trait]
impl RadStorageBackend for FileSystemBackend {
    async fn put(&self, key: &str, data: &str) -> Result<(), String> {
        let path = self.rad_dir.join(key);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        fs::write(&path, data)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<String>, String> {
        let path = self.rad_dir.join(key);

        if !path.exists() {
            return Ok(None);
        }

        fs::read_to_string(&path)
            .map(Some)
            .map_err(|e| format!("Failed to read file: {}", e))
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, String> {
        let prefix_path = self.rad_dir.join(prefix);
        let mut results = Vec::new();

        // Walk directory tree from the prefix base directory
        let base_dir = if prefix_path.exists() && prefix_path.is_dir() {
            prefix_path.clone()
        } else {
            prefix_path.parent()
                .unwrap_or(self.rad_dir.as_path())
                .to_path_buf()
        };

        if !base_dir.exists() {
            return Ok(results);
        }

        walk_dir(&base_dir, &self.rad_dir, prefix, &mut results)?;

        results.sort();
        Ok(results)
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        let path = self.rad_dir.join(key);

        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete file: {}", e))?;
        }

        Ok(())
    }
}

/// Recursively walk directory and collect paths matching prefix
fn walk_dir(dir: &Path, base: &Path, prefix: &str, results: &mut Vec<String>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            walk_dir(&path, base, prefix, results)?;
        } else {
            // Convert to relative path from base
            let rel_path = path.strip_prefix(base)
                .map_err(|e| format!("Failed to get relative path: {}", e))?;

            let rel_str = rel_path.to_str()
                .ok_or_else(|| "Invalid UTF-8 in path".to_string())?
                .replace('\\', "/"); // Normalize path separators

            // Check if it matches the prefix
            if rel_str.starts_with(prefix) {
                results.push(rel_str);
            }
        }
    }

    Ok(())
}

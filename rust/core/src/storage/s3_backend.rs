use async_trait::async_trait;
use s3::Bucket;
use s3::creds::Credentials;
use s3::Region;
use super::backend::RadStorageBackend;

pub struct S3Config {
    pub endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
}

/// S3-compatible storage backend.
/// Works with AWS S3, Cloudflare R2, Backblaze B2, iDrive, MinIO, etc.
pub struct S3Backend {
    bucket: Box<Bucket>,
}

impl S3Backend {
    pub fn new(config: S3Config) -> Result<Self, String> {
        let credentials = Credentials::new(
            Some(&config.access_key),
            Some(&config.secret_key),
            None,
            None,
            None,
        ).map_err(|e| format!("Failed to create credentials: {}", e))?;

        let region = Region::Custom {
            region: config.region.clone(),
            endpoint: config.endpoint.clone(),
        };

        let bucket = Bucket::new(&config.bucket, region, credentials)
            .map_err(|e| format!("Failed to create bucket: {}", e))?
            .with_path_style();

        Ok(Self { bucket })
    }
}

#[async_trait]
impl RadStorageBackend for S3Backend {
    async fn put(&self, key: &str, data: &str) -> Result<(), String> {
        println!("S3 PUT: bucket={}, key={}, data_len={}",
            self.bucket.name(), key, data.len());

        match self.bucket.put_object(key, data.as_bytes()).await {
            Ok(response) => {
                println!("S3 PUT success: status={}", response.status_code());
                Ok(())
            }
            Err(e) => {
                eprintln!("S3 PUT failed for {}: {}", key, e);
                Err(format!("S3 PUT failed: {}", e))
            }
        }
    }

    async fn get(&self, key: &str) -> Result<Option<String>, String> {
        match self.bucket.get_object(key).await {
            Ok(response) => {
                let data = String::from_utf8(response.bytes().to_vec())
                    .map_err(|e| format!("Invalid UTF-8: {}", e))?;
                Ok(Some(data))
            }
            Err(e) => {
                let err_str = e.to_string();
                // Check if it's a 404 error or other benign error
                if err_str.contains("404") || err_str.contains("NoSuchKey") || err_str.contains("NoSuchBucket") {
                    Ok(None)
                } else {
                    // For initialization, treat errors as "key not found"
                    eprintln!("S3 GET warning for key '{}': {}", key, e);
                    Ok(None)
                }
            }
        }
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, String> {
        match self.bucket.list(prefix.to_string(), None).await {
            Ok(results) => {
                let mut keys = Vec::new();
                for list in results {
                    for obj in list.contents {
                        keys.push(obj.key);
                    }
                }
                keys.sort();
                Ok(keys)
            }
            Err(e) => {
                // If list fails (e.g., empty bucket, parsing error), return empty list
                // This allows initialization with empty S3 bucket
                eprintln!("S3 LIST warning for prefix '{}': {}", prefix, e);
                Ok(Vec::new())
            }
        }
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        self.bucket
            .delete_object(key)
            .await
            .map_err(|e| format!("S3 DELETE failed: {}", e))?;

        Ok(())
    }
}

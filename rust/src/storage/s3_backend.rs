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
            .map_err(|e| format!("Failed to create bucket: {}", e))?;

        Ok(Self { bucket })
    }
}

#[async_trait]
impl RadStorageBackend for S3Backend {
    async fn put(&self, key: &str, data: &str) -> Result<(), String> {
        self.bucket
            .put_object(key, data.as_bytes())
            .await
            .map_err(|e| format!("S3 PUT failed: {}", e))?;

        Ok(())
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
                // Check if it's a 404 error
                if err_str.contains("404") || err_str.contains("NoSuchKey") {
                    Ok(None)
                } else {
                    Err(format!("S3 GET failed: {}", e))
                }
            }
        }
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, String> {
        let results = self.bucket
            .list(prefix.to_string(), None)
            .await
            .map_err(|e| format!("S3 LIST failed: {}", e))?;

        let mut keys = Vec::new();
        for list in results {
            for obj in list.contents {
                keys.push(obj.key);
            }
        }

        keys.sort();
        Ok(keys)
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        self.bucket
            .delete_object(key)
            .await
            .map_err(|e| format!("S3 DELETE failed: {}", e))?;

        Ok(())
    }
}

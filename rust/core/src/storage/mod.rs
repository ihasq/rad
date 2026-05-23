// Storage backends require async_trait which is not available without the feature
#[cfg(feature = "storage-backends")]
pub mod backend;
#[cfg(feature = "storage-backends")]
pub mod fs_backend;
#[cfg(feature = "storage-backends")]
pub mod s3_backend;
#[cfg(feature = "storage-backends")]
pub mod s3_store;

#[cfg(feature = "storage-backends")]
pub use backend::RadStorageBackend;
#[cfg(feature = "storage-backends")]
pub use fs_backend::FileSystemBackend;
#[cfg(feature = "storage-backends")]
pub use s3_backend::{S3Backend, S3Config};
#[cfg(feature = "storage-backends")]
pub use s3_store::S3RadStore;

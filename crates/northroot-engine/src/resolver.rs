//! Privacy-preserving resolver API for reconciling proof index with tenant artifacts.
//!
//! This module provides traits for artifact resolution and optional caching.
//! Tenants implement these traits; the engine only depends on the trait interfaces,
//! never concrete implementations. This ensures privacy: the proof index only sees
//! encrypted locators, never actual storage locations.
//!
//! ## Design Principles
//!
//! - **Privacy by design**: Receipts contain encrypted locators, not plain storage URIs
//! - **Tenant control**: Resolver implementations stay in tenant's trust boundary
//! - **Optional caching**: Managed cache is a paid add-on, not required
//! - **Batch operations**: Support efficient batch resolution for performance

use northroot_receipts::EncryptedLocatorRef;
use serde_json;
use std::fmt;

/// Error type for resolver operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolverError {
    /// Decryption failed
    DecryptionFailed(String),
    /// Storage operation failed
    StorageError(String),
    /// Invalid encrypted locator format
    InvalidLocator(String),
    /// Content hash mismatch
    HashMismatch { expected: String, actual: String },
    /// Resolver not configured
    NotConfigured(String),
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolverError::DecryptionFailed(msg) => {
                write!(f, "Decryption failed: {}", msg)
            }
            ResolverError::StorageError(msg) => {
                write!(f, "Storage error: {}", msg)
            }
            ResolverError::InvalidLocator(msg) => {
                write!(f, "Invalid locator: {}", msg)
            }
            ResolverError::HashMismatch { expected, actual } => {
                write!(
                    f,
                    "Content hash mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            ResolverError::NotConfigured(msg) => {
                write!(f, "Resolver not configured: {}", msg)
            }
        }
    }
}

impl std::error::Error for ResolverError {}

/// Error type for cache operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheError {
    /// Cache operation failed
    OperationFailed(String),
    /// Cache not available
    NotAvailable(String),
    /// TTL expired
    Expired,
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheError::OperationFailed(msg) => {
                write!(f, "Cache operation failed: {}", msg)
            }
            CacheError::NotAvailable(msg) => {
                write!(f, "Cache not available: {}", msg)
            }
            CacheError::Expired => {
                write!(f, "Cache entry expired")
            }
        }
    }
}

impl std::error::Error for CacheError {}

/// Artifact location (private to tenant, never in receipts)
///
/// This type represents the actual storage location of an artifact.
/// It is only used within the tenant's resolver implementation and
/// never exposed to the proof index or stored in receipts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactLocation {
    /// Storage type: "s3", "gcs", "local", "azure", etc.
    pub storage_type: String,
    /// Location path/URI (e.g., "s3://bucket/key", "/path/to/file")
    pub location: String,
    /// Content hash for verification (sha256 format: "sha256:<64hex>")
    pub content_hash: String,
    /// Optional metadata (credentials, region, etc.) - tenant-controlled
    pub metadata: Option<serde_json::Value>,
}

/// Artifact metadata for storage
///
/// Metadata about an artifact that is used when storing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactMetadata {
    /// MIME type (e.g., "application/parquet", "application/json")
    pub mime_type: Option<String>,
    /// Size in bytes
    pub size_bytes: u64,
    /// Optional tenant-specific metadata
    pub custom: Option<serde_json::Value>,
}

/// Privacy-preserving resolver for reconciling proof index with tenant artifacts
///
/// This trait is implemented by tenants and stays private. Never exposed publicly.
/// The proof index only sees encrypted locators, never actual storage locations.
///
/// ## Implementation Notes
///
/// - Tenants implement this trait with their own encryption/decryption logic
/// - The engine only depends on the trait interface, never concrete implementations
/// - Resolver implementations stay in the tenant's trust boundary
/// - Batch operations should be more efficient than individual calls
pub trait ArtifactResolver: Send + Sync {
    /// Decrypt and resolve encrypted locator to actual storage location
    ///
    /// Returns location information that stays in tenant's control.
    /// Proof index never sees this - only the resolver implementation.
    ///
    /// # Arguments
    ///
    /// * `encrypted_ref` - Encrypted locator reference from receipt
    ///
    /// # Returns
    ///
    /// Artifact location with storage type, path, and content hash
    ///
    /// # Errors
    ///
    /// Returns error if decryption fails, locator is invalid, or storage is inaccessible
    fn resolve_locator(
        &self,
        encrypted_ref: &EncryptedLocatorRef,
    ) -> Result<ArtifactLocation, ResolverError>;

    /// Encrypt and store artifact, return encrypted locator reference
    ///
    /// Tenant stores artifact in their storage and returns encrypted reference
    /// for inclusion in receipts.
    ///
    /// # Arguments
    ///
    /// * `data` - Artifact data bytes
    /// * `metadata` - Artifact metadata (MIME type, size, etc.)
    ///
    /// # Returns
    ///
    /// Encrypted locator reference for inclusion in receipt
    ///
    /// # Errors
    ///
    /// Returns error if storage fails or encryption fails
    fn store_artifact(
        &self,
        data: &[u8],
        metadata: &ArtifactMetadata,
    ) -> Result<EncryptedLocatorRef, ResolverError>;

    /// Batch resolve (for efficiency)
    ///
    /// Resolves multiple encrypted locators in a single operation.
    /// Should be more efficient than calling `resolve_locator` multiple times.
    ///
    /// # Arguments
    ///
    /// * `encrypted_refs` - Slice of encrypted locator references
    ///
    /// # Returns
    ///
    /// Vector of artifact locations in the same order as input
    ///
    /// # Errors
    ///
    /// Returns error if any resolution fails (implementation may choose to fail fast or continue)
    fn resolve_locators_batch(
        &self,
        encrypted_refs: &[EncryptedLocatorRef],
    ) -> Result<Vec<ArtifactLocation>, ResolverError>;
}

/// Optional managed artifact cache (paid add-on)
///
/// Northroot can optionally cache hot shards for speed.
/// Tenant can opt-in to managed cache for frequently accessed artifacts.
///
/// ## Implementation Notes
///
/// - This is an optional paid add-on service
/// - Default behavior: defer on demand via tenant resolver
/// - Cache implementations are provided by northroot, not tenants
/// - TTL (time-to-live) is optional; None means cache indefinitely
pub trait ManagedCache: Send + Sync {
    /// Store artifact in managed cache
    ///
    /// # Arguments
    ///
    /// * `artifact_ref` - Encrypted locator reference (used as cache key)
    /// * `data` - Artifact data to cache
    /// * `ttl` - Optional time-to-live in seconds (None = cache indefinitely)
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Errors
    ///
    /// Returns error if cache operation fails
    fn cache_artifact(
        &self,
        artifact_ref: &EncryptedLocatorRef,
        data: &[u8],
        ttl: Option<u64>,
    ) -> Result<(), CacheError>;

    /// Retrieve artifact from managed cache
    ///
    /// # Arguments
    ///
    /// * `artifact_ref` - Encrypted locator reference (used as cache key)
    ///
    /// # Returns
    ///
    /// Some(data) if cached and not expired, None if not found or expired
    ///
    /// # Errors
    ///
    /// Returns error if cache operation fails
    fn get_cached_artifact(
        &self,
        artifact_ref: &EncryptedLocatorRef,
    ) -> Result<Option<Vec<u8>>, CacheError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_error_display() {
        let err = ResolverError::DecryptionFailed("test error".to_string());
        assert!(err.to_string().contains("Decryption failed"));
    }

    #[test]
    fn test_cache_error_display() {
        let err = CacheError::OperationFailed("test error".to_string());
        assert!(err.to_string().contains("Cache operation failed"));
    }

    #[test]
    fn test_artifact_location() {
        let location = ArtifactLocation {
            storage_type: "s3".to_string(),
            location: "s3://bucket/key".to_string(),
            content_hash: "sha256:test".to_string(),
            metadata: None,
        };
        assert_eq!(location.storage_type, "s3");
        assert_eq!(location.location, "s3://bucket/key");
    }

    #[test]
    fn test_artifact_metadata() {
        let metadata = ArtifactMetadata {
            mime_type: Some("application/parquet".to_string()),
            size_bytes: 1024,
            custom: None,
        };
        assert_eq!(metadata.mime_type, Some("application/parquet".to_string()));
        assert_eq!(metadata.size_bytes, 1024);
    }
}

//! Helper functions for data shape hash computation.
//!
//! This module provides convenience functions for computing data shape hashes
//! from various input sources (files, bytes, multiple inputs) for PAC key computation.
//!
//! This is part of ADR-0009-P08: Helper Functions for Shape Hash Computation.

use std::fs;
use std::path::Path;

use northroot_receipts::EncryptedLocatorRef;

use crate::cas::build_manifest_from_data;
use crate::shapes::{compute_data_shape_hash, ChunkScheme, DataShape, DataShapeError};

/// Compute data shape hash from a file.
///
/// Reads the file, chunks it, builds a manifest, and computes the data shape hash.
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `chunk_scheme` - Chunking scheme to use (default: CDC with avg_size=65536)
///
/// # Returns
///
/// Data shape hash in format `sha256:<64hex>`
///
/// # Errors
///
/// Returns error if file cannot be read or shape computation fails
///
/// # Example
///
/// ```rust
/// use northroot_engine::delta::compute_data_shape_hash_from_file;
/// use northroot_engine::ChunkScheme;
///
/// let hash = compute_data_shape_hash_from_file(
///     "data.csv",
///     Some(ChunkScheme::CDC { avg_size: 65536 })
/// ).unwrap();
/// ```
pub fn compute_data_shape_hash_from_file<P: AsRef<Path>>(
    path: P,
    chunk_scheme: Option<ChunkScheme>,
) -> Result<String, DataShapeError> {
    // Read file
    let data = fs::read(path.as_ref())
        .map_err(|e| DataShapeError::serialization(
            format!("Failed to read file: {}", e),
            format!("path: {:?}", path.as_ref()),
        ))?;

    compute_data_shape_hash_from_bytes(&data, chunk_scheme)
}

/// Compute data shape hash from bytes.
///
/// Chunks the bytes, builds a manifest, and computes the data shape hash.
///
/// # Arguments
///
/// * `data` - Input data bytes
/// * `chunk_scheme` - Chunking scheme to use (default: CDC with avg_size=65536)
///
/// # Returns
///
/// Data shape hash in format `sha256:<64hex>`
///
/// # Errors
///
/// Returns error if manifest building or shape computation fails
///
/// # Example
///
/// ```rust
/// use northroot_engine::delta::compute_data_shape_hash_from_bytes;
/// use northroot_engine::ChunkScheme;
///
/// let data = b"hello, world";
/// let hash = compute_data_shape_hash_from_bytes(
///     data,
///     Some(ChunkScheme::Fixed { size: 4096 })
/// ).unwrap();
/// ```
pub fn compute_data_shape_hash_from_bytes(
    data: &[u8],
    chunk_scheme: Option<ChunkScheme>,
) -> Result<String, DataShapeError> {
    // Use default chunk scheme if not provided
    let scheme = chunk_scheme.unwrap_or(ChunkScheme::CDC { avg_size: 65536 });

    // Build manifest from data
    let manifest = build_manifest_from_data(data, scheme.clone())
        .map_err(|e| DataShapeError::serialization(
            format!("Failed to build manifest: {}", e),
            format!("data_size: {}, chunk_scheme: {:?}", data.len(), scheme),
        ))?;

    // Create DataShape::ByteStream from manifest
    let shape = DataShape::ByteStream {
        manifest_root: manifest.manifest_root.clone(),
        manifest_len: manifest.manifest_len,
        chunk_scheme: scheme,
    };

    // Compute hash
    compute_data_shape_hash(&shape)
}

/// Compute composite data shape hash from multiple encrypted locators.
///
/// This function creates a composite data shape representing multiple inputs.
/// Since we can't decrypt the locators (they're tenant-encrypted), we create
/// a composite shape that represents the combination of inputs.
///
/// The composite shape uses a Merkle root over the locator content hashes.
///
/// # Arguments
///
/// * `locators` - Vector of encrypted locator references
///
/// # Returns
///
/// Composite data shape hash in format `sha256:<64hex>`
///
/// # Errors
///
/// Returns error if shape computation fails
///
/// # Example
///
/// ```rust
/// use northroot_engine::delta::compute_data_shape_hash_from_inputs;
/// use northroot_receipts::EncryptedLocatorRef;
///
/// let locators = vec![
///     EncryptedLocatorRef {
///         encrypted_data: vec![1, 2, 3],
///         content_hash: "sha256:abc123...".to_string(),
///         encryption_scheme: "aes256-gcm".to_string(),
///     },
/// ];
/// let hash = compute_data_shape_hash_from_inputs(&locators).unwrap();
/// ```
pub fn compute_data_shape_hash_from_inputs(
    locators: &[EncryptedLocatorRef],
) -> Result<String, DataShapeError> {
    if locators.is_empty() {
        return Err(DataShapeError::serialization(
            "Cannot compute shape hash from empty input list",
            format!("locators.len(): {}", locators.len()),
        ));
    }

    // Extract content hashes from locators
    let content_hashes: Vec<&str> = locators
        .iter()
        .map(|loc| loc.content_hash.as_str())
        .collect();

    // Compute Merkle root over content hashes
    use crate::delta::manifest_root::compute_manifest_root_from_strings;
    let manifest_root_bytes = compute_manifest_root_from_strings(content_hashes.iter().cloned())
        .map_err(|e| {
            DataShapeError::serialization(
                format!("Failed to compute manifest root: {}", e),
                format!("content_hashes.len(): {}", content_hashes.len()),
            )
        })?;

    // Convert to hex string
    let manifest_root = format!("sha256:{}", hex::encode(manifest_root_bytes));

    // Create composite DataShape::ByteStream
    let shape = DataShape::ByteStream {
        manifest_root,
        manifest_len: locators.len() as u64, // Approximate: number of inputs
        chunk_scheme: ChunkScheme::Fixed { size: 1 }, // Placeholder for composite
    };

    // Compute hash
    compute_data_shape_hash(&shape)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compute_data_shape_hash_from_bytes() {
        let data = b"hello, world";
        let hash = compute_data_shape_hash_from_bytes(data, None).unwrap();
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 71);
    }

    #[test]
    fn test_compute_data_shape_hash_from_file() {
        let mut file = NamedTempFile::new().unwrap();
        use std::io::Write;
        file.write_all(b"test data").unwrap();
        file.flush().unwrap();

        let hash = compute_data_shape_hash_from_file(file.path(), None).unwrap();
        assert!(hash.starts_with("sha256:"));
    }

    #[test]
    fn test_compute_data_shape_hash_from_inputs() {
        let locators = vec![
            EncryptedLocatorRef {
                encrypted_data: vec![1, 2, 3],
                content_hash:
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                        .to_string(),
                encryption_scheme: "aes256-gcm".to_string(),
            },
            EncryptedLocatorRef {
                encrypted_data: vec![4, 5, 6],
                content_hash:
                    "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                        .to_string(),
                encryption_scheme: "aes256-gcm".to_string(),
            },
        ];

        let hash = compute_data_shape_hash_from_inputs(&locators).unwrap();
        assert!(hash.starts_with("sha256:"));
    }

    #[test]
    fn test_compute_data_shape_hash_from_inputs_empty() {
        let locators = vec![];
        let result = compute_data_shape_hash_from_inputs(&locators);
        assert!(result.is_err());
    }
}

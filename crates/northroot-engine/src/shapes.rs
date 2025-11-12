//! Data shape representation for unified proof structure.
//!
//! This module provides the `DataShape` enum that represents data uniformly
//! as byte streams while exposing row-aware overlays for delta reasoning.
//! Everything is bytes at the substrate; semantic overlays added when helpful.
//!
//! ## Design Principles
//!
//! - **Unify substrate**: Everything is bytes; add semantic overlays when helpful
//! - **Portable reuse**: Decisions driven by PAC keys, never raw data
//! - **Engine-internal**: DataShape enum stays in engine; receipts store only hashes
//!
//! ## Usage
//!
//! ```rust
//! use northroot_engine::shapes::{DataShape, ChunkScheme, KeyFormat, RowValueRepr};
//!
//! // ByteStream example
//! let bytestream = DataShape::ByteStream {
//!     manifest_root: "sha256:abc...".to_string(),
//!     manifest_len: 1024,
//!     chunk_scheme: ChunkScheme::CDC { avg_size: 65536 },
//! };
//!
//! // RowMap example
//! let rowmap = DataShape::RowMap {
//!     merkle_root: "sha256:def...".to_string(),
//!     row_count: 1000,
//!     key_fmt: KeyFormat::Sha256Hex,
//!     value_repr: RowValueRepr::Number,
//! };
//!
//! // Compute hash for receipt
//! let hash = compute_data_shape_hash(&bytestream);
//! ```

use crate::commitments::{jcs, sha256_prefixed};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

/// Data shape: unified representation of data for proof computation
///
/// Everything is bytes at the substrate; semantic overlays added when helpful.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataShape {
    /// ByteStream: storage-level view (fast, universal)
    ByteStream {
        /// Merkle root over chunk list (RFC-6962 style)
        manifest_root: String, // sha256:<64hex>
        /// Total bytes
        manifest_len: u64,
        /// Chunking scheme
        chunk_scheme: ChunkScheme,
    },
    /// RowMap: semantic view for structured compute (fine-grained deltas)
    RowMap {
        /// Merkle Row-Map root over {k -> v_or_ptr}
        merkle_root: String, // sha256:<64hex>
        /// Number of rows
        row_count: u64,
        /// Key format (e.g., sha256:<64hex>)
        key_fmt: KeyFormat,
        /// Value representation
        value_repr: RowValueRepr,
    },
}

/// Chunking scheme for ByteStream
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChunkScheme {
    /// Content-Defined Chunking (Rabin fingerprinting)
    CDC { avg_size: u64 },
    /// Fixed-size chunks
    Fixed { size: u64 },
}

/// Key format for RowMap
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyFormat {
    /// SHA-256 hash format
    Sha256Hex,
}

/// Value representation for RowMap
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowValueRepr {
    /// Numeric value
    Number,
    /// String value
    String,
    /// Pointer to ByteStream chunk
    Pointer,
}

/// Error type for data shape operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DataShapeError {
    /// Invalid hash format
    #[error("Invalid hash format: {message}. Expected format: sha256:<64hex>, got: {provided}")]
    InvalidHashFormat {
        /// Error message
        message: String,
        /// Provided hash value
        provided: String,
    },
    /// Serialization error
    #[error("Serialization failed: {message}. Context: {context}")]
    Serialization {
        /// Error message
        message: String,
        /// Additional context
        context: String,
    },
}

impl DataShapeError {
    /// Create an invalid hash format error with context
    pub fn invalid_hash_format(message: impl Into<String>, provided: impl Into<String>) -> Self {
        Self::InvalidHashFormat {
            message: message.into(),
            provided: provided.into(),
        }
    }

    /// Create a serialization error with context
    pub fn serialization(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
            context: context.into(),
        }
    }
}

/// Compute data shape hash from DataShape enum
///
/// Returns sha256:<64hex> format for use in receipts.
/// Uses engine-local JCS (per `commitments.rs`) for serialization.
///
/// # Arguments
///
/// * `shape` - DataShape enum to hash
///
/// # Returns
///
/// Hash string in format `sha256:<64hex>`
///
/// # Errors
///
/// Returns error if serialization fails
pub fn compute_data_shape_hash(shape: &DataShape) -> Result<String, DataShapeError> {
    let canonical = match shape {
        DataShape::ByteStream {
            manifest_root,
            manifest_len,
            chunk_scheme,
        } => {
            json!({
                "kind": "bytestream",
                "manifest_root": manifest_root,
                "manifest_len": manifest_len,
                "chunk_scheme": chunk_scheme_to_json(chunk_scheme),
            })
        }
        DataShape::RowMap {
            merkle_root,
            row_count,
            key_fmt,
            value_repr,
        } => {
            json!({
                "kind": "rowmap",
                "merkle_root": merkle_root,
                "row_count": row_count,
                "key_fmt": key_fmt_to_json(key_fmt),
                "value_repr": value_repr_to_json(value_repr),
            })
        }
    };

    // JCS canonicalization (sorted keys) for engine-internal computation
    let jcs_bytes = jcs(&canonical);
    Ok(sha256_prefixed(jcs_bytes.as_bytes()))
}

/// Convert ChunkScheme to JSON value for serialization
fn chunk_scheme_to_json(scheme: &ChunkScheme) -> serde_json::Value {
    match scheme {
        ChunkScheme::CDC { avg_size } => {
            json!({
                "type": "cdc",
                "avg_size": avg_size,
            })
        }
        ChunkScheme::Fixed { size } => {
            json!({
                "type": "fixed",
                "size": size,
            })
        }
    }
}

/// Convert KeyFormat to JSON value for serialization
fn key_fmt_to_json(fmt: &KeyFormat) -> serde_json::Value {
    match fmt {
        KeyFormat::Sha256Hex => json!("sha256_hex"),
    }
}

/// Convert RowValueRepr to JSON value for serialization
fn value_repr_to_json(repr: &RowValueRepr) -> serde_json::Value {
    match repr {
        RowValueRepr::Number => json!("number"),
        RowValueRepr::String => json!("string"),
        RowValueRepr::Pointer => json!("pointer"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_data_shape_hash_bytestream() {
        let shape = DataShape::ByteStream {
            manifest_root:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
            manifest_len: 1024,
            chunk_scheme: ChunkScheme::CDC { avg_size: 65536 },
        };

        let hash = compute_data_shape_hash(&shape).unwrap();
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 71); // "sha256:" + 64 hex chars
    }

    #[test]
    fn test_compute_data_shape_hash_rowmap() {
        let shape = DataShape::RowMap {
            merkle_root: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                .to_string(),
            row_count: 1000,
            key_fmt: KeyFormat::Sha256Hex,
            value_repr: RowValueRepr::Number,
        };

        let hash = compute_data_shape_hash(&shape).unwrap();
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 71);
    }

    #[test]
    fn test_data_shape_hash_deterministic() {
        let shape1 = DataShape::ByteStream {
            manifest_root: "sha256:abc123".to_string(),
            manifest_len: 1024,
            chunk_scheme: ChunkScheme::Fixed { size: 4096 },
        };

        let shape2 = DataShape::ByteStream {
            manifest_root: "sha256:abc123".to_string(),
            manifest_len: 1024,
            chunk_scheme: ChunkScheme::Fixed { size: 4096 },
        };

        let hash1 = compute_data_shape_hash(&shape1).unwrap();
        let hash2 = compute_data_shape_hash(&shape2).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_data_shape_hash_different_shapes() {
        let bytestream = DataShape::ByteStream {
            manifest_root: "sha256:abc123".to_string(),
            manifest_len: 1024,
            chunk_scheme: ChunkScheme::CDC { avg_size: 65536 },
        };

        let rowmap = DataShape::RowMap {
            merkle_root: "sha256:abc123".to_string(),
            row_count: 1024,
            key_fmt: KeyFormat::Sha256Hex,
            value_repr: RowValueRepr::Number,
        };

        let hash1 = compute_data_shape_hash(&bytestream).unwrap();
        let hash2 = compute_data_shape_hash(&rowmap).unwrap();
        assert_ne!(hash1, hash2);
    }
}

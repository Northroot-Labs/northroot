//! Proof-addressable storage backend for receipts and manifests.
//!
//! This crate provides storage backends for persisting receipts and manifests
//! with support for content-addressed lookup, compression, and retention policies.

pub mod error;
pub mod sqlite;
pub mod traits;

pub use error::StorageError;
pub use sqlite::SqliteStore;
pub use traits::{ManifestMeta, ReceiptQuery, ReceiptStore};


//! Pluggable storage backend abstraction for Northroot events.
//!
//! This crate provides:
//! - `StoreWriter` and `StoreReader` traits for append-only event storage
//! - Default journal-backed implementation using `northroot-journal`
//! - Extensible design for future backends (S3, in-memory, etc.)
//!
//! The journal backend is the reference implementation and follows the
//! format specified in `docs/FORMAT.md`.

#![deny(missing_docs)]

/// Error types for store operations.
pub mod error;
/// Journal-backed storage implementation.
pub mod journal;
/// Storage backend traits.
pub mod traits;

pub use error::StoreError;
pub use journal::{JournalBackendReader, JournalBackendWriter};
pub use northroot_journal::{EventJson, ReadMode, WriteOptions};
pub use traits::{StoreReader, StoreWriter};


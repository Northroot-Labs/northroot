//! Pluggable storage backend abstraction for Northroot events.
//!
//! This crate provides:
//! - `StoreWriter` and `StoreReader` traits for append-only event storage
//! - Default journal-backed implementation using `northroot-journal`
//! - Event filtering API for selective iteration
//! - Typed event parsing and view helpers
//! - Extensible design for future backends (S3, in-memory, etc.)
//!
//! The journal backend is the reference implementation and follows the
//! format specified in `docs/reference/format.md`.

#![deny(missing_docs)]

/// Error types for store operations.
pub mod error;
/// Event filtering API.
pub mod filter;
/// Journal-backed storage implementation.
pub mod journal;
/// Storage backend traits.
pub mod traits;
/// Typed event parsing.
pub mod typed;
/// View API for event linkage navigation.
pub mod view;

pub use error::StoreError;
pub use filter::{
    AndFilter, EventFilter, EventIdFilter, EventTypeFilter, FilteredReader, OrFilter,
    PrincipalFilter, TimeRangeFilter,
};
pub use journal::{JournalBackendReader, JournalBackendWriter};
pub use northroot_journal::{EventJson, ReadMode, WriteOptions};
pub use traits::{StoreReader, StoreWriter};
pub use typed::{parse_event, ParseError, TypedEvent};
pub use view::{executions_for_auth, resolve_auth};

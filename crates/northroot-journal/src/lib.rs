//! Append-only journal format for canonical Northroot events.
//!
//! This crate provides:
//! - Framed, append-only storage for canonical event JSON
//! - Reader/writer APIs with strict and permissive modes
//! - Verification hooks for event identity validation
//!
//! The journal format is specified in `docs/reference/format.md`.

#![deny(missing_docs)]

/// Error types for journal operations.
pub mod errors;
/// Event JSON type alias and helpers.
pub mod event;
/// Frame structure and serialization.
pub mod frame;
/// Journal reader implementation.
pub mod reader;
/// Verification helpers for journal events.
pub mod verification;
/// Journal writer implementation.
pub mod writer;

pub use errors::JournalError;
pub use event::EventJson;
pub use frame::{FrameKind, JournalHeader, RecordFrame};
pub use reader::{JournalReader, ReadMode};
pub use verification::{verify_event, verify_event_id};
pub use writer::{JournalWriter, WriteOptions};

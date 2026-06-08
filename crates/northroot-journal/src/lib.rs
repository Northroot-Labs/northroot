//! Append-only journal format for canonical Northroot events.
//!
//! This crate provides:
//! - Framed, append-only storage for canonical event JSON
//! - Reader/writer APIs with strict and permissive modes
//! - Verification hooks for event identity validation
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use northroot_canonical::{compute_event_id, Canonicalizer, ProfileId};
//! use northroot_journal::{JournalWriter, JournalReader, WriteOptions, ReadMode};
//! use serde_json::json;
//!
//! // Create canonicalizer
//! let profile = ProfileId::parse("northroot-canonical-v1")?;
//! let canonicalizer = Canonicalizer::new(profile);
//!
//! // Write an event
//! let mut event = json!({
//!     "event_type": "test",
//!     "event_version": "1",
//!     "occurred_at": "2024-01-01T00:00:00Z",
//!     "principal_id": "service:example",
//!     "canonical_profile_id": "northroot-canonical-v1"
//! });
//!
//! let event_id = compute_event_id(&event, &canonicalizer)?;
//! event["event_id"] = serde_json::to_value(&event_id)?;
//!
//! let mut writer = JournalWriter::open("events.nrj", WriteOptions::default())?;
//! writer.append_event(&event)?;
//! writer.finish()?;
//!
//! // Read events
//! let mut reader = JournalReader::open("events.nrj", ReadMode::Strict)?;
//! while let Some(read_event) = reader.read_event()? {
//!     println!("Read event: {}", read_event["event_id"]);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Key Types
//!
//! - [`JournalWriter`] - Write events to journal files
//! - [`JournalReader`] - Read events from journal files
//! - [`verify_event_id`] - Verify event identity
//!
//! ## See Also
//!
//! - [API Documentation](https://docs.rs/northroot-journal) - Full API reference
//! - [Journal Format Reference](../../../docs/reference/format.md) - Format specification
//! - [Core Specification](../../../docs/reference/spec.md) - Protocol details
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
pub use event::{EventJson, EventObject};
pub use frame::{FrameKind, JournalHeader, RecordFrame};
pub use reader::{JournalReader, ReadMode};
pub use verification::verify_event_id;
pub use writer::{JournalWriter, WriteOptions};

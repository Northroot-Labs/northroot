//! Core event types, verification, and receipt structures for Northroot.
//!
//! This crate provides:
//! - Event types matching `schemas/events/v1/*.schema.json`
//! - Event ID computation via domain-separated hashing
//! - Offline verification logic for event consistency and bounds
//! - Verification verdicts (Ok, Denied, Violation, Invalid)
//!
//! Core invariants:
//! - Events are immutable, append-only evidence records
//! - Event IDs are content-derived: `H(domain_separator || canonical_bytes(event))`
//! - Verification is deterministic and offline
//! - Core does not execute or decide outcomes; it only verifies evidence
//!
#![deny(missing_docs)]

/// Event types matching the event schemas.
pub mod events;
/// Event ID computation with domain-separated hashing.
pub mod event_id;
/// Verification logic and verdict types.
pub mod verification;
/// Shared types used across events (Meter, ResourceRef, etc.).
pub mod shared;
/// Error types for core operations.
pub mod errors;

pub use events::{
    AuthorizationEvent, AuthorizationKind, CheckResult, Decision, ExecutionEvent, Outcome,
    AttestationEvent, CheckpointEvent,
};
pub use event_id::{compute_event_id, EventIdError};
pub use verification::{VerificationVerdict, Verifier};
pub use shared::{Meter, ResourceRef, IntentAnchors};
pub use errors::CoreError;


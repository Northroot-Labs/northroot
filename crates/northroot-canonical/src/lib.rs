//! Canonical data model primitives for Northroot verifiable events and proof envelopes.
//!
//! This crate provides deterministic canonicalization, event identity computation, and
//! lossless numeric types for the Northroot trust kernel.
//!
//! ## Overview
//!
//! These types mirror `schemas/canonical/v1/types.schema.json` and the
//! canonicalization profile described in `docs/reference/canonicalization.md`.
//! Every field that participates in hashing or verification lives in this crate.
//!
//! ## Quick Start
//!
//! ```rust
//! use northroot_canonical::{Canonicalizer, ProfileId, compute_event_id};
//! use serde_json::json;
//!
//! // Create a canonicalizer
//! let profile = ProfileId::parse("northroot-canonical-v1")?;
//! let canonicalizer = Canonicalizer::new(profile);
//!
//! // Canonicalize JSON
//! let value = json!({"b": 2, "a": 1});
//! let result = canonicalizer.canonicalize(&value)?;
//! // result.bytes contains deterministic canonical bytes
//!
//! // Compute event ID
//! let event = json!({
//!     "event_type": "test",
//!     "event_version": "1",
//!     "occurred_at": "2024-01-01T00:00:00Z",
//!     "principal_id": "service:example",
//!     "canonical_profile_id": "northroot-canonical-v1"
//! });
//! let event_id = compute_event_id(&event, &canonicalizer)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Key Types
//!
//! - [`Canonicalizer`] - Produces deterministic canonical bytes from JSON
//! - [`compute_event_id`] - Computes content-derived event identifiers
//! - [`Quantity`] - Lossless numeric types (Dec, Int, Rat, F64)
//! - [`Digest`] - Content-addressed identifiers
//! - [`PrincipalId`], [`ProfileId`], [`Timestamp`] - Core identifier types
//!
//! ## See Also
//!
//! - [API Documentation](https://docs.rs/northroot-canonical) - Full API reference
//! - [Canonicalization Reference](../../../docs/reference/canonicalization.md) - Detailed canonicalization rules
//! - [Core Specification](../../../docs/reference/spec.md) - Protocol specification
//!
#![deny(missing_docs)]

/// Canonicalization helpers for deterministic hashing.
pub mod canonicalizer;
/// Digest/identifier primitives.
pub mod digest;
/// Event ID computation with domain-separated hashing.
pub mod event_id;
/// Hygiene report types emitted during canonicalization.
pub mod hygiene;
/// Core identifiers and newtypes derived from canonical schema.
pub mod identifiers;
/// Strict JSON parsing for canonical evidence boundaries.
pub mod json;
/// Quantity types (Dec, Int, Rat, F64) encoded per canonical profile.
pub mod quantities;
/// Validation helpers used by canonical types.
pub mod validation;

pub use canonicalizer::{CanonicalizationError, CanonicalizationResult, Canonicalizer};
pub use digest::{compute_blob_digest, Digest, DigestAlg};
pub use event_id::{compute_event_id, verify_event_id, EventIdError};
pub use hygiene::{HygieneReport, HygieneStatus, HygieneWarning};
pub use identifiers::{ContentRef, PrincipalId, ProfileId, Timestamp, ToolName};
pub use json::{parse_json_strict, StrictJsonError};
pub use quantities::Quantity;
pub use validation::ValidationError;

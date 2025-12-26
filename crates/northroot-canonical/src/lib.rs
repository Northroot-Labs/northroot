//! Canonical data model primitives for Northroot events and receipts.
//!
//! These types mirror `schemas/canonical/v1/types.schema.json` and the
//! canonicalization profile described in `docs/reference/canonicalization.md`.
//! Every field that participates in hashing or verification lives in this crate.
//!
#![deny(missing_docs)]

/// Canonicalization helpers for deterministic hashing.
pub mod canonicalizer;
/// Digest/identifier primitives.
pub mod digest;
/// Hygiene report types emitted during canonicalization.
pub mod hygiene;
/// Core identifiers and newtypes derived from canonical schema.
pub mod identifiers;
/// Quantity types (Dec, Int, Rat, F64) encoded per canonical profile.
pub mod quantities;
/// Validation helpers used by canonical types.
pub mod validation;

pub use canonicalizer::{CanonicalizationError, CanonicalizationResult, Canonicalizer};
pub use digest::{Digest, DigestAlg};
pub use hygiene::{HygieneReport, HygieneStatus, HygieneWarning};
pub use identifiers::{ContentRef, PrincipalId, ProfileId, Timestamp, ToolName};
pub use quantities::Quantity;
pub use validation::ValidationError;

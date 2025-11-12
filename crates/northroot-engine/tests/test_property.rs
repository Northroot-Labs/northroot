//! Property tests for critical invariants and edge cases.
//!
//! This module contains comprehensive property-based tests using proptest to ensure
//! production robustness. Tests are organized into:
//! - Invariants: Determinism, order independence, idempotency
//! - Edge cases: Empty inputs, boundary conditions, malformed data, stress tests

#[path = "property/generators.rs"]
mod generators;

#[path = "property/invariants.rs"]
mod invariants;

#[path = "property/edge_cases.rs"]
mod edge_cases;


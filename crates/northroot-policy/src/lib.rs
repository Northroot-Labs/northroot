//! Northroot Policy — Policies and strategies.
//!
//! This crate defines policies and strategies for the Northroot proof algebra system,
//! including cost models, reuse thresholds, allow/deny rules, and floating-point tolerances.

#![deny(missing_docs)]

pub mod validation;

pub use validation::{
    load_policy, validate_determinism, validate_policy, validate_policy_ref_format,
    validate_region_constraints, validate_tool_constraints, PolicyError,
};

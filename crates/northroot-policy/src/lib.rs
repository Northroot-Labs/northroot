//! Northroot Policy — Policies and strategies.
//!
//! This crate defines policies and strategies for the Northroot proof algebra system,
//! including cost models, reuse thresholds, allow/deny rules, and floating-point tolerances.

#![deny(missing_docs)]

pub mod cost_model;
pub mod policy;
pub mod validation;

pub use cost_model::{CostModel, CostValue};
pub use policy::{
    CostModelDefinition, DecisionBounds, DecisionRule, DeltaComputePolicy, OverlapConfig,
    PolicyConstraints,
};
pub use validation::{
    extract_cost_model, load_policy, validate_determinism, validate_policy,
    validate_policy_ref_format, validate_region_constraints, validate_tool_constraints,
    PolicyError,
};

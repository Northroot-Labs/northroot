//! Delta compute policy types.
//!
//! This module defines the structure for delta compute policies that can be
//! loaded from JSON files and used to drive reuse decisions.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Overlap measurement configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlapConfig {
    /// How to measure overlap: "jaccard:row-hash", "sketch:minhash:128", etc.
    pub measure: String,
    /// Minimum sample size (0 = exact)
    pub min_sample: u64,
    /// Tolerance for overlap comparison
    pub tolerance: f64,
}

/// Decision rule configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionRule {
    /// Decision rule formula (e.g., "j > c_id/(alpha*c_comp)")
    pub rule: String,
    /// Fallback decision if metrics unavailable
    pub fallback: String,
    /// Bounds for row counts
    pub bounds: Option<DecisionBounds>,
}

/// Decision bounds for row counts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionBounds {
    /// Minimum row count
    pub min_rows: u64,
    /// Maximum row count
    pub max_rows: u64,
}

/// Policy constraints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyConstraints {
    /// Forbid NaN values
    pub forbid_nan: Option<bool>,
    /// Header policy: "require", "optional", "forbid"
    pub header_policy: Option<String>,
    /// Row hash scheme: "sha256-per-row", etc.
    pub row_hash_scheme: Option<String>,
}

/// Cost model definition in policy JSON.
///
/// This is the JSON structure for cost_model in policy files.
/// It will be converted to a concrete CostModel via extract_cost_model().
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostModelDefinition {
    /// Identity/integration cost definition
    pub c_id: Value,
    /// Baseline compute cost definition
    pub c_comp: Value,
    /// Operator incrementality factor definition
    pub alpha: Value,
}

/// Delta compute policy structure.
///
/// This matches the policy JSON structure from the spec:
/// `docs/specs/incremental_compute.md` lines 100-126
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeltaComputePolicy {
    /// Schema version: "policy.delta.v1"
    pub schema_version: String,
    /// Policy identifier: "acme/reuse_thresholds@1"
    pub policy_id: String,
    /// Determinism requirement: "strict", "bounded", "observational"
    pub determinism: Option<String>,
    /// Overlap measurement configuration
    pub overlap: OverlapConfig,
    /// Cost model definition (JSON structure)
    pub cost_model: CostModelDefinition,
    /// Decision rule configuration
    pub decision: DecisionRule,
    /// Optional policy constraints
    pub constraints: Option<PolicyConstraints>,
}

impl DeltaComputePolicy {
    /// Create a new delta compute policy.
    pub fn new(
        schema_version: String,
        policy_id: String,
        determinism: Option<String>,
        overlap: OverlapConfig,
        cost_model: CostModelDefinition,
        decision: DecisionRule,
        constraints: Option<PolicyConstraints>,
    ) -> Self {
        Self {
            schema_version,
            policy_id,
            determinism,
            overlap,
            cost_model,
            decision,
            constraints,
        }
    }
}

impl TryFrom<Value> for DeltaComputePolicy {
    type Error = String;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value).map_err(|e| format!("Failed to parse policy: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_policy_from_json() {
        let json = json!({
            "schema_version": "policy.delta.v1",
            "policy_id": "acme/reuse_thresholds@1",
            "determinism": "strict",
            "overlap": {
                "measure": "jaccard:row-hash",
                "min_sample": 0,
                "tolerance": 0.0
            },
            "cost_model": {
                "c_id": { "type": "constant", "value": 1.0 },
                "c_comp": { "type": "linear", "per_row": 0.00001, "base": 0.5 },
                "alpha": { "type": "constant", "value": 0.9 }
            },
            "decision": {
                "rule": "j > c_id/(alpha*c_comp)",
                "fallback": "recompute",
                "bounds": { "min_rows": 1, "max_rows": 1000000000 }
            },
            "constraints": {
                "forbid_nan": true,
                "header_policy": "require",
                "row_hash_scheme": "sha256-per-row"
            }
        });

        let policy: DeltaComputePolicy = json.try_into().unwrap();
        assert_eq!(policy.schema_version, "policy.delta.v1");
        assert_eq!(policy.policy_id, "acme/reuse_thresholds@1");
        assert_eq!(policy.determinism, Some("strict".to_string()));
        assert_eq!(policy.overlap.measure, "jaccard:row-hash");
        assert_eq!(policy.decision.rule, "j > c_id/(alpha*c_comp)");
    }

    #[test]
    fn test_policy_minimal() {
        let json = json!({
            "schema_version": "policy.delta.v1",
            "policy_id": "test/policy@1",
            "overlap": {
                "measure": "jaccard:row-hash",
                "min_sample": 0,
                "tolerance": 0.0
            },
            "cost_model": {
                "c_id": { "type": "constant", "value": 1.0 },
                "c_comp": { "type": "constant", "value": 100.0 },
                "alpha": { "type": "constant", "value": 0.9 }
            },
            "decision": {
                "rule": "j > c_id/(alpha*c_comp)",
                "fallback": "recompute"
            }
        });

        let policy: DeltaComputePolicy = json.try_into().unwrap();
        assert_eq!(policy.determinism, None);
        assert_eq!(policy.constraints, None);
    }
}

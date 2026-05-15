//! Obligation primitive for accountable Northroot work.

use northroot_core::{require_non_empty, require_prefix, NorthrootResult, ValidatePrimitive};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Durable unit of accountable work.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Obligation {
    /// Schema version.
    pub schema_version: String,
    /// Obligation identifier.
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Responsible owner actor.
    pub owner: String,
    /// Assigned actor.
    pub assignee: String,
    /// Lifecycle status.
    pub status: ObligationStatus,
    /// Objective payload.
    pub objective: Value,
    /// Execution constraints.
    #[serde(default)]
    pub constraints: BTreeMap<String, Value>,
    /// Verification requirements.
    pub verification: VerificationSpec,
}

/// Obligation lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObligationStatus {
    /// Created but not accepted.
    Created,
    /// Accepted by the assignee.
    Accepted,
    /// Planned.
    Planned,
    /// Running.
    Running,
    /// Blocked.
    Blocked,
    /// Completed.
    Completed,
    /// Failed.
    Failed,
    /// Cancelled.
    Cancelled,
    /// Evidence reconciled.
    Reconciled,
}

/// Required evidence classes for closure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationSpec {
    /// Required receipt claim or evidence kinds.
    #[serde(default)]
    pub required_receipts: Vec<String>,
}

impl ValidatePrimitive for Obligation {
    fn validate(&self) -> NorthrootResult<()> {
        require_non_empty("schema_version", &self.schema_version)?;
        require_prefix("id", &self.id, "obl_")?;
        require_non_empty("title", &self.title)?;
        require_prefix("owner", &self.owner, "actor:")?;
        require_prefix("assignee", &self.assignee, "actor:")?;
        if !self.objective.is_object() {
            return Err(northroot_core::NorthrootError::InvalidValue {
                field: "objective",
                value: self.objective.to_string(),
            });
        }
        if self.verification.required_receipts.is_empty() {
            return Err(northroot_core::NorthrootError::MissingField {
                field: "verification.required_receipts",
            });
        }
        for receipt in &self.verification.required_receipts {
            require_non_empty("verification.required_receipts", receipt)?;
        }
        Ok(())
    }
}

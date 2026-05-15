//! Policy primitive for Northroot authority boundaries.

use northroot_core::{require_non_empty, require_prefix, NorthrootResult, ValidatePrimitive};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Machine-readable constraint set over execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Policy {
    /// Schema version.
    pub schema_version: String,
    /// Policy identifier.
    pub id: String,
    /// Applicability selector.
    #[serde(default)]
    pub applies_to: BTreeMap<String, Value>,
    /// Policy rules.
    #[serde(default)]
    pub rules: Vec<Value>,
}

impl ValidatePrimitive for Policy {
    fn validate(&self) -> NorthrootResult<()> {
        require_non_empty("schema_version", &self.schema_version)?;
        require_prefix("id", &self.id, "policy:")?;
        if self.rules.is_empty() {
            return Err(northroot_core::NorthrootError::MissingField { field: "rules" });
        }
        for rule in &self.rules {
            if !rule.is_object() {
                return Err(northroot_core::NorthrootError::InvalidValue {
                    field: "rules",
                    value: rule.to_string(),
                });
            }
        }
        Ok(())
    }
}

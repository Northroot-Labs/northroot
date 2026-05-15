//! Execution event primitive for Northroot JSONL logs.

use northroot_canonical::{compute_event_id, Digest};
use northroot_core::{
    default_canonicalizer, require_non_empty, require_prefix, NorthrootResult, ValidatePrimitive,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Append-only event emitted during execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionEvent {
    /// Content-derived event identifier. Optional before finalization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<Digest>,
    /// Event type, such as `tool.invoked`.
    pub event_type: String,
    /// Event schema version.
    pub event_version: String,
    /// RFC3339 timestamp.
    pub timestamp: String,
    /// Obligation identifier.
    pub obligation_id: String,
    /// Actor identifier.
    pub actor_id: String,
    /// Optional tool name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// Optional input hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_hash: Option<String>,
    /// Optional output hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_hash: Option<String>,
    /// Optional cost payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<Value>,
    /// Extension metadata.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
}

impl ExecutionEvent {
    /// Compute the content-derived event id for this event.
    pub fn compute_event_id(&self) -> NorthrootResult<Digest> {
        Ok(compute_event_id(self, &default_canonicalizer()?)?)
    }
}

impl ValidatePrimitive for ExecutionEvent {
    fn validate(&self) -> NorthrootResult<()> {
        require_non_empty("event_type", &self.event_type)?;
        require_non_empty("event_version", &self.event_version)?;
        require_non_empty("timestamp", &self.timestamp)?;
        require_prefix("obligation_id", &self.obligation_id, "obl_")?;
        require_prefix("actor_id", &self.actor_id, "actor:")?;
        if let Some(tool) = &self.tool {
            require_non_empty("tool", tool)?;
        }
        Ok(())
    }
}

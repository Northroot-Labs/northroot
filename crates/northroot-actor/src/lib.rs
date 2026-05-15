//! Actor primitive for accountable Northroot work.

use northroot_core::{require_non_empty, require_prefix, NorthrootResult, ValidatePrimitive};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Entity that can hold responsibility or execute actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    /// Schema version.
    pub schema_version: String,
    /// Stable actor identifier.
    pub id: String,
    /// Actor kind.
    pub kind: ActorKind,
    /// Human-readable actor name.
    pub display_name: String,
    /// Explicit authority scope entries.
    #[serde(default)]
    pub authority_scope: Vec<String>,
    /// Optional metadata.
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

/// Actor categories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    /// Human operator.
    Human,
    /// Agent process.
    Agent,
    /// Service account.
    ServiceAccount,
    /// GitHub app or similar installed integration.
    GitHubApp,
    /// Workflow runner.
    WorkflowRunner,
    /// Organization actor.
    Organization,
    /// Client or external system.
    ClientSystem,
}

impl ValidatePrimitive for Actor {
    fn validate(&self) -> NorthrootResult<()> {
        require_non_empty("schema_version", &self.schema_version)?;
        require_prefix("id", &self.id, "actor:")?;
        require_non_empty("display_name", &self.display_name)?;
        for scope in &self.authority_scope {
            require_non_empty("authority_scope", scope)?;
        }
        Ok(())
    }
}

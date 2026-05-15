//! Cost attribution primitive for Northroot executions.

use northroot_core::{require_non_empty, require_prefix, NorthrootResult, ValidatePrimitive};
use serde::{Deserialize, Serialize};

/// Economic metadata for work executed under Northroot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostAttribution {
    /// Provider name, such as `openai`, `anthropic`, or `github`.
    pub provider: String,
    /// Provider product or service.
    pub product: String,
    /// Unit kind, such as `usd`, `token`, `credit`, or `minute`.
    pub unit_kind: String,
    /// Unit count encoded as a string to avoid float drift.
    pub units: String,
    /// Estimated USD amount encoded as a string.
    pub estimated_usd: String,
    /// Pricing snapshot identifier.
    pub pricing_snapshot: String,
    /// Attribution target.
    pub attributed_to: CostTarget,
}

/// Attribution links for an execution cost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostTarget {
    /// Obligation identifier.
    pub obligation: String,
    /// Actor identifier.
    pub actor: String,
    /// Project identifier.
    pub project: String,
}

impl ValidatePrimitive for CostAttribution {
    fn validate(&self) -> NorthrootResult<()> {
        require_non_empty("provider", &self.provider)?;
        require_non_empty("product", &self.product)?;
        require_non_empty("unit_kind", &self.unit_kind)?;
        require_non_empty("units", &self.units)?;
        require_non_empty("estimated_usd", &self.estimated_usd)?;
        require_non_empty("pricing_snapshot", &self.pricing_snapshot)?;
        require_prefix(
            "attributed_to.obligation",
            &self.attributed_to.obligation,
            "obl_",
        )?;
        require_prefix("attributed_to.actor", &self.attributed_to.actor, "actor:")?;
        require_non_empty("attributed_to.project", &self.attributed_to.project)?;
        Ok(())
    }
}

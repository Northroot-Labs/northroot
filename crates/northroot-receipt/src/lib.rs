//! Receipt primitive for Northroot claim-plus-evidence proofs.

use northroot_core::{require_non_empty, require_prefix, NorthrootResult, ValidatePrimitive};
use northroot_cost::CostAttribution;
use serde::{Deserialize, Serialize};

/// Signed or hash-verifiable claim over completed work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// Schema version.
    pub schema_version: String,
    /// Receipt identifier.
    pub receipt_id: String,
    /// Obligation identifier.
    pub obligation_id: String,
    /// Claim being made.
    pub claim: String,
    /// Actor making the claim.
    pub actor_id: String,
    /// Evidence supporting the claim.
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
    /// Result status.
    pub result: ReceiptResult,
    /// Creation timestamp.
    pub created_at: String,
    /// Optional cost attribution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostAttribution>,
}

/// Receipt result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptResult {
    /// Claim passed.
    Passed,
    /// Claim failed.
    Failed,
    /// Claim is blocked.
    Blocked,
}

/// Evidence reference bound to a receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// Evidence kind.
    pub kind: String,
    /// Evidence URI.
    pub uri: String,
    /// Expected evidence hash.
    pub hash: String,
}

impl ValidatePrimitive for Receipt {
    fn validate(&self) -> NorthrootResult<()> {
        require_non_empty("schema_version", &self.schema_version)?;
        require_prefix("receipt_id", &self.receipt_id, "rcpt_")?;
        require_prefix("obligation_id", &self.obligation_id, "obl_")?;
        require_non_empty("claim", &self.claim)?;
        require_prefix("actor_id", &self.actor_id, "actor:")?;
        require_non_empty("created_at", &self.created_at)?;
        if self.evidence.is_empty() {
            return Err(northroot_core::NorthrootError::MissingField { field: "evidence" });
        }
        for item in &self.evidence {
            item.validate()?;
        }
        if let Some(cost) = &self.cost {
            cost.validate()?;
        }
        Ok(())
    }
}

impl ValidatePrimitive for EvidenceRef {
    fn validate(&self) -> NorthrootResult<()> {
        require_non_empty("evidence.kind", &self.kind)?;
        require_non_empty("evidence.uri", &self.uri)?;
        require_prefix("evidence.hash", &self.hash, "sha256:")?;
        Ok(())
    }
}

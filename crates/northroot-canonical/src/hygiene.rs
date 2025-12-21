use crate::identifiers::ProfileId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Hygiene status for canonicalization attempts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HygieneStatus {
    /// The input was canonicalizable without issues.
    Ok,
    /// The input was lossy but accepted; warnings should be inspected.
    Lossy,
    /// The input was ambiguous (e.g., whitespace normalization required).
    Ambiguous,
    /// The input was invalid and must be rejected.
    Invalid,
}

/// Stable warning code emitted by canonicalization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HygieneWarning(String);

impl HygieneWarning {
    /// Creates a warning from a literal code.
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }
}

impl AsRef<str> for HygieneWarning {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Hygiene reports produced during canonicalization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HygieneReport {
    /// Overall hygiene status.
    pub status: HygieneStatus,
    /// Stable warning codes.
    pub warnings: Vec<HygieneWarning>,
    /// Metrics such as duplicate key counts.
    pub metrics: BTreeMap<String, u64>,
    /// Identifier of the canonicalization profile that produced the bytes.
    pub profile_id: ProfileId,
}

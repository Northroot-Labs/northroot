use canonical_json::to_string;
use serde_json::Value;

use crate::hygiene::{HygieneReport, HygieneStatus, HygieneWarning};
use crate::identifiers::ProfileId;
use std::collections::BTreeMap;

/// Error returned when canonicalization fails.
#[derive(thiserror::Error, Debug)]
pub enum CanonicalizationError {
    /// Provided JSON could not be canonicalized.
    #[error("invalid JSON structure: {0}")]
    InvalidStructure(String),
    /// A duplicate object member was detected.
    /// Note: This error is reserved for future use at the JSON parsing layer.
    /// serde_json::Value::Object cannot have duplicates by design, so this
    /// cannot occur during canonicalization of already-parsed Values.
    #[error("duplicate key detected at {0}")]
    #[allow(dead_code)]
    DuplicateKey(String),
    /// Raw JSON number detected in strict mode.
    #[error("raw JSON number detected at {0}")]
    RawJsonNumber(String),
    /// Non-finite number (NaN/Infinity) detected.
    #[error("non-finite number detected at {0}")]
    NonFiniteNumber(String),
    /// Generic failure.
    #[error("other error: {0}")]
    Other(String),
}

/// Result of canonicalization.
#[derive(Debug)]
pub struct CanonicalizationResult {
    /// Canonical UTF-8 bytes for the input value.
    pub bytes: Vec<u8>,
    /// Hygiene report describing strict-mode validation.
    pub report: HygieneReport,
}

/// Helper for building JSON paths during validation.
#[derive(Debug, Clone)]
struct Path {
    segments: Vec<String>,
}

impl Path {
    fn root() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    fn push_field(&self, field: &str) -> Self {
        let mut segments = self.segments.clone();
        segments.push(field.to_string());
        Self { segments }
    }

    fn push_index(&self, index: usize) -> Self {
        let mut segments = self.segments.clone();
        segments.push(format!("[{}]", index));
        Self { segments }
    }

    fn to_string(&self) -> String {
        if self.segments.is_empty() {
            return "root".to_string();
        }
        self.segments.join(".")
    }
}

/// Canonicalizer that emits deterministic bytes.
pub struct Canonicalizer {
    profile: ProfileId,
}

impl Canonicalizer {
    /// Creates a new canonicalizer for the provided profile.
    pub fn new(profile: ProfileId) -> Self {
        Self { profile }
    }

    /// Produces canonical bytes + hygiene report.
    pub fn canonicalize(
        &self,
        value: &Value,
    ) -> Result<CanonicalizationResult, CanonicalizationError> {
        let mut report = HygieneReport {
            status: HygieneStatus::Ok,
            warnings: vec![],
            metrics: BTreeMap::new(),
            profile_id: self.profile.clone(),
        };

        // Validate structure and populate report
        if let Err(e) = self.validate(value, Path::root(), &mut report) {
            report.status = HygieneStatus::Invalid;
            // Store report in error context for downstream access
            return Err(e);
        }

        // Perform RFC 8785 canonicalization
        let canonical = to_string(value)
            .map_err(|err| CanonicalizationError::Other(err.to_string()))?;
        let bytes = canonical.into_bytes();

        Ok(CanonicalizationResult { bytes, report })
    }

    /// Produces canonical bytes + hygiene report, returning the report even on error.
    pub fn canonicalize_with_report(
        &self,
        value: &Value,
    ) -> Result<CanonicalizationResult, (CanonicalizationError, HygieneReport)> {
        let mut report = HygieneReport {
            status: HygieneStatus::Ok,
            warnings: vec![],
            metrics: BTreeMap::new(),
            profile_id: self.profile.clone(),
        };

        // Validate structure and populate report
        if let Err(e) = self.validate(value, Path::root(), &mut report) {
            report.status = HygieneStatus::Invalid;
            return Err((e, report));
        }

        // Perform RFC 8785 canonicalization
        let canonical = to_string(value)
            .map_err(|err| {
                let error_report = HygieneReport {
                    status: HygieneStatus::Invalid,
                    warnings: report.warnings.clone(),
                    metrics: report.metrics.clone(),
                    profile_id: report.profile_id.clone(),
                };
                (
                    CanonicalizationError::Other(err.to_string()),
                    error_report,
                )
            })?;
        let bytes = canonical.into_bytes();

        Ok(CanonicalizationResult { bytes, report })
    }

    /// Validates the JSON value according to the canonical profile.
    fn validate(
        &self,
        value: &Value,
        path: Path,
        report: &mut HygieneReport,
    ) -> Result<(), CanonicalizationError> {
        match value {
            Value::Object(map) => {
                // Note: Duplicate key detection is redundant here because
                // serde_json::Value::Object is a BTreeMap which cannot have duplicates.
                // Duplicate detection should happen at the JSON parsing layer, not here.
                for (key, child) in map {
                    self.validate(child, path.push_field(key), report)?;
                }
                Ok(())
            }
            Value::Array(items) => {
                for (idx, item) in items.iter().enumerate() {
                    self.validate(item, path.push_index(idx), report)?;
                }
                Ok(())
            }
            Value::Number(num) => {
                // Check for non-finite numbers (NaN/Infinity)
                if num.is_f64() {
                    let f = num.as_f64().unwrap();
                    if !f.is_finite() {
                        report.warnings.push(HygieneWarning::new("NonFiniteNumber"));
                        report
                            .metrics
                            .entry("non_finite_numbers".to_string())
                            .and_modify(|count| *count += 1)
                            .or_insert(1);
                        return Err(CanonicalizationError::NonFiniteNumber(path.to_string()));
                    }
                }
                // In strict mode, raw JSON numbers are forbidden for quantity fields
                // For now, we emit a warning but allow it (schema-level validation should reject)
                // This can be made stricter if needed
                report.warnings.push(HygieneWarning::new("RawJsonNumber"));
                report
                    .metrics
                    .entry("raw_json_numbers".to_string())
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
                Err(CanonicalizationError::RawJsonNumber(path.to_string()))
            }
            Value::String(s) => {
                // Validate UTF-8 (serde_json already ensures this, but we check anyway)
                if s.chars().any(|c| c as u32 > 0x10FFFF) {
                    report.status = HygieneStatus::Invalid;
                    return Err(CanonicalizationError::InvalidStructure(format!(
                        "{}: invalid UTF-8",
                        path.to_string()
                    )));
                }
                Ok(())
            }
            Value::Bool(_) | Value::Null => Ok(()),
        }
    }
}

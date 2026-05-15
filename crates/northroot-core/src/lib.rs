//! Shared canonical hashing and validation helpers for Northroot primitives.

use northroot_canonical::{Canonicalizer, ProfileId};
use serde_json::Value;
use sha2::{Digest as Sha2Digest, Sha256};
use std::fs;
use std::path::Path;

/// Canonicalization profile used by the first public Northroot examples.
pub const CANONICAL_PROFILE_ID: &str = "northroot-canonical-v1";

/// Shared result type for Northroot primitive validation.
pub type NorthrootResult<T> = Result<T, NorthrootError>;

/// Shared error type for primitive crates and CLI validation.
#[derive(Debug, thiserror::Error)]
pub enum NorthrootError {
    /// A required field is missing or empty.
    #[error("{field} is required")]
    MissingField {
        /// Field name.
        field: &'static str,
    },
    /// A field has an invalid value.
    #[error("{field} has invalid value: {value}")]
    InvalidValue {
        /// Field name.
        field: &'static str,
        /// Invalid value.
        value: String,
    },
    /// JSON parsing or serialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// I/O failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Canonicalization failed.
    #[error("canonicalization failed: {0}")]
    Canonicalization(#[from] northroot_canonical::CanonicalizationError),
    /// Canonical identifier validation failed.
    #[error("canonical validation failed: {0}")]
    CanonicalValidation(#[from] northroot_canonical::ValidationError),
    /// Event identity computation failed.
    #[error("event id computation failed: {0}")]
    EventId(#[from] northroot_canonical::EventIdError),
}

/// Trait implemented by schema primitives that can validate themselves.
pub trait ValidatePrimitive {
    /// Validate the primitive.
    fn validate(&self) -> NorthrootResult<()>;
}

/// Build the default canonicalizer.
pub fn default_canonicalizer() -> NorthrootResult<Canonicalizer> {
    let profile = ProfileId::parse(CANONICAL_PROFILE_ID)?;
    Ok(Canonicalizer::new(profile))
}

/// Return canonical bytes for a JSON value.
pub fn canonical_bytes(value: &Value) -> NorthrootResult<Vec<u8>> {
    Ok(default_canonicalizer()?.canonicalize(value)?.bytes)
}

/// Return `sha256:<hex>` for raw bytes.
pub fn raw_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

/// Return `sha256:<hex>` for canonical JSON bytes.
pub fn canonical_sha256(value: &Value) -> NorthrootResult<String> {
    Ok(raw_sha256(&canonical_bytes(value)?))
}

/// Return `sha256:<hex>` for a JSONL stream after canonicalizing each JSON object line.
pub fn jsonl_sha256(input: &str) -> NorthrootResult<String> {
    let mut bytes = Vec::new();
    for line in input.lines().filter(|line| !line.trim().is_empty()) {
        let value: Value = serde_json::from_str(line)?;
        bytes.extend(canonical_bytes(&value)?);
        bytes.push(b'\n');
    }
    Ok(raw_sha256(&bytes))
}

/// Hash a JSON, JSONL, or raw file using the default Northroot rules.
pub fn file_sha256(path: &Path) -> NorthrootResult<String> {
    let input = fs::read_to_string(path);
    match (
        path.extension().and_then(|ext| ext.to_str()),
        input.as_deref(),
    ) {
        (Some("json"), Ok(text)) => {
            let value: Value = serde_json::from_str(text)?;
            canonical_sha256(&value)
        }
        (Some("jsonl" | "ndjson"), Ok(text)) => jsonl_sha256(text),
        _ => Ok(raw_sha256(&fs::read(path)?)),
    }
}

/// Read a JSON file into a value.
pub fn read_json(path: &Path) -> NorthrootResult<Value> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

/// Validate that a field is not empty.
pub fn require_non_empty(field: &'static str, value: &str) -> NorthrootResult<()> {
    if value.trim().is_empty() {
        return Err(NorthrootError::MissingField { field });
    }
    Ok(())
}

/// Validate that an identifier has the expected prefix.
pub fn require_prefix(field: &'static str, value: &str, prefix: &str) -> NorthrootResult<()> {
    require_non_empty(field, value)?;
    if !value.starts_with(prefix) {
        return Err(NorthrootError::InvalidValue {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

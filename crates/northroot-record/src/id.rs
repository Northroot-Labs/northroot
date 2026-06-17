use crate::record::Record;
use northroot_canonical::{Canonicalizer, ProfileId};
use serde_json::Value;
use sha2::{Digest as Sha2Digest, Sha256};
use std::fmt;

/// Error returned while computing or verifying a record identifier.
#[derive(thiserror::Error, Debug)]
pub enum RecordIdError {
    /// Serialization failed.
    #[error("serialization failed: {0}")]
    Serialization(String),
    /// Canonicalization failed.
    #[error("canonicalization failed: {0}")]
    Canonicalization(#[from] northroot_canonical::CanonicalizationError),
    /// Content identifier is malformed.
    #[error("malformed content id: {0}")]
    MalformedContentId(String),
}

const RECORD_DOMAIN_SEPARATOR: &[u8] = b"northroot:record:v0\0";

/// Public Northroot content identifier in `sha256:<64 lowercase hex>` form.
///
/// This compact string form is used for record IDs and references. Structured
/// digest objects may still be useful inside richer metadata, but public record
/// identifiers remain single string values for logs, refs, URLs, and manifests.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ContentId(String);

impl ContentId {
    /// Parses and validates a content identifier.
    ///
    /// # Errors
    ///
    /// Returns [`RecordIdError`] when the identifier is not lowercase SHA-256 hex.
    pub fn parse(value: impl Into<String>) -> Result<Self, RecordIdError> {
        let value = value.into();
        if is_content_id(&value) {
            Ok(Self(value))
        } else {
            Err(RecordIdError::MalformedContentId(value))
        }
    }

    /// Returns the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Returns canonical bytes for a record, excluding the self-referential `id`.
///
/// # Errors
///
/// Returns [`RecordIdError`] if serialization or canonicalization fails.
pub fn record_canonical_bytes(record: &Record) -> Result<Vec<u8>, RecordIdError> {
    let mut value: Value = serde_json::to_value(record)
        .map_err(|err| RecordIdError::Serialization(err.to_string()))?;
    if let Value::Object(map) = &mut value {
        map.remove("id");
    }
    let profile = ProfileId::parse("northroot-canonical-v1")
        .map_err(|err| RecordIdError::Serialization(err.to_string()))?;
    let canonicalizer = Canonicalizer::new(profile);
    Ok(canonicalizer.canonicalize(&value)?.bytes)
}

/// Computes a `sha256:<hex>` content identifier for a record.
///
/// # Errors
///
/// Returns [`RecordIdError`] if serialization or canonicalization fails.
pub fn compute_record_id(record: &Record) -> Result<String, RecordIdError> {
    let canonical = record_canonical_bytes(record)?;
    let mut hasher = Sha256::new();
    hasher.update(RECORD_DOMAIN_SEPARATOR);
    hasher.update(canonical);
    let digest = hasher.finalize();
    Ok(format!("sha256:{}", hex_lower(&digest)))
}

/// Returns true when `value` is a lowercase SHA-256 content identifier.
pub fn is_content_id(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Verifies that `record.id` equals the computed content identifier.
///
/// # Errors
///
/// Returns [`RecordIdError`] if serialization or canonicalization fails.
pub fn verify_record_id(record: &Record) -> Result<bool, RecordIdError> {
    Ok(record.id == compute_record_id(record)?)
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("sha256:{}", hex_lower(&digest))
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("writing to string cannot fail");
    }
    out
}

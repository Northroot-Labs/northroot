//! HashRef v0 durable identifier helpers.
//!
//! These helpers own proof-promotable receipt and grant hashing. Runtime and
//! agent adapters may implement compatible code, but must pass the shared
//! golden vectors.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest as Sha2Digest, Sha256};

use crate::{Canonicalizer, ValidationError};

const RECEIPT_DOMAIN_SEPARATOR: &[u8] = b"northroot:receipt:v0\0";
const GRANT_DOMAIN_SEPARATOR: &[u8] = b"northroot:grant:v0\0";
const RECORD_DOMAIN_SEPARATOR: &[u8] = b"northroot:record:v0\0";
const TOKEN_REF_DOMAIN_SEPARATOR: &[u8] = b"northroot:token-ref:v0\0";

/// Computes a durable receipt id from canonical receipt bytes.
pub fn compute_receipt_id_from_canonical_bytes(bytes: &[u8]) -> String {
    prefixed_hash("nr_receipt", RECEIPT_DOMAIN_SEPARATOR, bytes)
}

/// Computes a durable grant id from canonical grant bytes.
pub fn compute_grant_id_from_canonical_bytes(bytes: &[u8]) -> String {
    prefixed_hash("nr_grant", GRANT_DOMAIN_SEPARATOR, bytes)
}

/// Computes a durable local operational record id from canonical record bytes.
pub fn compute_record_id_from_canonical_bytes(bytes: &[u8]) -> String {
    prefixed_hash("nr_record", RECORD_DOMAIN_SEPARATOR, bytes)
}

/// Computes a non-secret credential reference from secret bytes.
pub fn compute_token_ref(secret: &[u8]) -> String {
    prefixed_hash("nr_token_ref", TOKEN_REF_DOMAIN_SEPARATOR, secret)
}

/// Computes a durable receipt id from a serializable receipt payload.
pub fn compute_receipt_id<T: Serialize>(
    receipt: &T,
    canonicalizer: &Canonicalizer,
) -> Result<String, HashRefError> {
    let bytes = canonical_bytes_without_id(receipt, canonicalizer, "receipt_id")?;
    Ok(compute_receipt_id_from_canonical_bytes(&bytes))
}

/// Computes a durable grant id from a serializable grant payload.
pub fn compute_grant_id<T: Serialize>(
    grant: &T,
    canonicalizer: &Canonicalizer,
) -> Result<String, HashRefError> {
    let bytes = canonical_bytes_without_id(grant, canonicalizer, "grant_id")?;
    Ok(compute_grant_id_from_canonical_bytes(&bytes))
}

/// Computes a durable operational record id from a serializable record payload.
pub fn compute_record_id<T: Serialize>(
    record: &T,
    canonicalizer: &Canonicalizer,
) -> Result<String, HashRefError> {
    let bytes = canonical_bytes_without_id(record, canonicalizer, "record_id")?;
    Ok(compute_record_id_from_canonical_bytes(&bytes))
}

fn canonical_bytes_without_id<T: Serialize>(
    value: &T,
    canonicalizer: &Canonicalizer,
    id_field: &str,
) -> Result<Vec<u8>, HashRefError> {
    let mut value: Value =
        serde_json::to_value(value).map_err(|e| HashRefError::Serialization(e.to_string()))?;
    if let Value::Object(map) = &mut value {
        map.remove(id_field);
    }
    Ok(canonicalizer.canonicalize(&value)?.bytes)
}

fn prefixed_hash(prefix: &str, domain_separator: &[u8], bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain_separator);
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("{prefix}_{}", lower_hex(&digest))
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Error during HashRef id computation.
#[derive(thiserror::Error, Debug)]
pub enum HashRefError {
    /// Serialization failed.
    #[error("serialization failed: {0}")]
    Serialization(String),
    /// Canonicalization failed.
    #[error("canonicalization failed: {0}")]
    Canonicalization(#[from] crate::CanonicalizationError),
    /// Digest construction failed.
    #[error("digest construction failed: {0}")]
    Digest(#[from] ValidationError),
}

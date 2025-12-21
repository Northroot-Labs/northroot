use northroot_canonical::{Canonicalizer, Digest, DigestAlg};
use sha2::{Digest as Sha2Digest, Sha256};
use serde::Serialize;
use serde_json::Value;

/// Domain separator for event ID computation: `b"northroot:event:v1\0"`.
const EVENT_DOMAIN_SEPARATOR: &[u8] = b"northroot:event:v1\0";

/// Computes the event ID for a canonical event.
///
/// Formula: `sha256(domain_separator || canonical_bytes(event))`
///
/// The event must be serializable and will be canonicalized before hashing.
pub fn compute_event_id<T: Serialize>(
    event: &T,
    canonicalizer: &Canonicalizer,
) -> Result<Digest, EventIdError> {
    // Serialize to JSON Value first
    let mut value: Value = serde_json::to_value(event)
        .map_err(|e| EventIdError::Serialization(e.to_string()))?;

    // Remove event_id to avoid self-referential hashing
    if let Value::Object(map) = &mut value {
        map.remove("event_id");
    }

    // Stringify all JSON numbers to satisfy canonicalizer hygiene rules
    stringify_numbers(&mut value);

    // Canonicalize the JSON value
    let result = canonicalizer.canonicalize(&value)?;
    
    // Hash: domain_separator || canonical_bytes
    let mut hasher = Sha256::new();
    hasher.update(EVENT_DOMAIN_SEPARATOR);
    hasher.update(&result.bytes);
    let hash_bytes = hasher.finalize();
    
    use base64::Engine;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash_bytes);
    Ok(Digest::new(DigestAlg::Sha256, b64)?)
}

/// Error during event ID computation.
#[derive(thiserror::Error, Debug)]
pub enum EventIdError {
    /// Serialization failed.
    #[error("serialization failed: {0}")]
    Serialization(String),
    /// Canonicalization failed.
    #[error("canonicalization failed: {0}")]
    Canonicalization(#[from] northroot_canonical::CanonicalizationError),
    /// Digest construction failed.
    #[error("digest construction failed: {0}")]
    Digest(#[from] northroot_canonical::ValidationError),
}

/// Recursively converts all JSON numbers into strings.
fn stringify_numbers(value: &mut Value) {
    match value {
        Value::Number(n) => {
            let s = n.to_string();
            *value = Value::String(s);
        }
        Value::Array(arr) => {
            for v in arr {
                stringify_numbers(v);
            }
        }
        Value::Object(map) => {
            for v in map.values_mut() {
                stringify_numbers(v);
            }
        }
        _ => {}
    }
}


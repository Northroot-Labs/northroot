//! Event ID computation with domain-separated hashing.
//!
//! Event IDs are computed as: `sha256(domain_separator || canonical_bytes(event))`
//! where the event_id field is excluded from the hash input.

use crate::{Canonicalizer, Digest, DigestAlg};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest as Sha2Digest, Sha256};

/// Domain separator for event ID computation: `b"northroot:event:v1\0"`.
const EVENT_DOMAIN_SEPARATOR: &[u8] = b"northroot:event:v1\0";

/// Computes the event ID for a canonical event.
///
/// Formula: `sha256(domain_separator || canonical_bytes(event))`
///
/// The event must be serializable and will be canonicalized before hashing.
/// The `event_id` field (if present) is excluded from the hash input.
///
/// # Example
///
/// ```rust
/// use northroot_canonical::{compute_event_id, Canonicalizer, ProfileId};
/// use serde_json::json;
///
/// let profile = ProfileId::parse("northroot-canonical-v1")?;
/// let canonicalizer = Canonicalizer::new(profile);
///
/// let event = json!({
///     "event_type": "test",
///     "event_version": "1",
///     "occurred_at": "2024-01-01T00:00:00Z",
///     "principal_id": "service:example",
///     "canonical_profile_id": "northroot-canonical-v1"
/// });
///
/// let event_id = compute_event_id(&event, &canonicalizer)?;
/// // event_id is a Digest that can be serialized or compared
/// assert_eq!(event_id.alg, northroot_canonical::DigestAlg::Sha256);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns [`EventIdError`] if serialization or canonicalization fails.
///
/// # See Also
///
/// - [`verify_event_id`] - Verify a claimed event ID
/// - [Event Model Documentation](../../../docs/reference/events.md) - Event structure
pub fn compute_event_id<T: Serialize>(
    event: &T,
    canonicalizer: &Canonicalizer,
) -> Result<Digest, EventIdError> {
    // Serialize to JSON Value first
    let mut value: Value =
        serde_json::to_value(event).map_err(|e| EventIdError::Serialization(e.to_string()))?;

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
    Canonicalization(#[from] crate::CanonicalizationError),
    /// Digest construction failed.
    #[error("digest construction failed: {0}")]
    Digest(#[from] crate::ValidationError),
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

/// Verifies that a claimed event_id matches the computed event_id.
///
/// Returns `true` if the claimed ID matches the computed ID, `false` otherwise.
///
/// # Example
///
/// ```rust
/// use northroot_canonical::{verify_event_id, compute_event_id, Canonicalizer, ProfileId, Digest};
/// use serde_json::json;
///
/// let profile = ProfileId::parse("northroot-canonical-v1")?;
/// let canonicalizer = Canonicalizer::new(profile);
///
/// let event = json!({
///     "event_type": "test",
///     "event_version": "1",
///     "occurred_at": "2024-01-01T00:00:00Z",
///     "principal_id": "service:example",
///     "canonical_profile_id": "northroot-canonical-v1"
/// });
///
/// let computed_id = compute_event_id(&event, &canonicalizer)?;
/// let is_valid = verify_event_id(&event, &computed_id, &canonicalizer)?;
/// assert!(is_valid);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns [`EventIdError`] if computation fails.
pub fn verify_event_id<T: Serialize>(
    event: &T,
    claimed_id: &Digest,
    canonicalizer: &Canonicalizer,
) -> Result<bool, EventIdError> {
    let computed_id = compute_event_id(event, canonicalizer)?;
    Ok(claimed_id == &computed_id)
}

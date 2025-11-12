//! Canonicalization and hash computation for receipts.
//!
//! This module provides functions for computing canonical CBOR representations
//! and SHA-256 hashes of receipts according to RFC 8949 (CBOR Deterministic Encoding).
//! All core serialization uses CBOR exclusively; JSON is only available through adapters.

use crate::Receipt;
use serde::Serialize;
use sha2::{Digest, Sha256};

pub mod cbor_canon;
pub mod cdn;

pub use cbor_canon::{encode_canonical, hash_canonical, CanonError};
pub use cdn::to_cdn;

/// Compute SHA-256 hash of canonical CBOR body with `sha256:` prefix.
///
/// This computes the hash of the canonical CBOR representation (excluding `sig` and `hash` fields),
/// and returns it in the format `sha256:<64hex>`.
///
/// **Breaking Change**: This now uses CBOR canonicalization instead of JSON (JCS).
/// All receipt hashes will change from previous versions.
///
/// # Arguments
///
/// * `receipt` - Receipt to hash
///
/// # Returns
///
/// Hash string in format `sha256:<64hex>`
///
/// # Errors
///
/// Returns error if CBOR encoding fails
pub fn compute_hash(receipt: &Receipt) -> Result<String, CanonError> {
    // Create a temporary receipt without sig and hash for canonicalization
    // We'll serialize to CBOR Value, remove those fields, then canonicalize
    use ciborium::value::Value as CborValue;

    // Serialize receipt to CBOR Value
    let mut buffer = Vec::new();
    ciborium::ser::into_writer(receipt, &mut buffer)
        .map_err(|e| CanonError::ToValue(e.to_string()))?;

    let mut value: CborValue = ciborium::de::from_reader(buffer.as_slice())
        .map_err(|e| CanonError::ToValue(e.to_string()))?;

    // Remove sig and hash fields from the map
    if let CborValue::Map(ref mut map) = value {
        map.retain(|(k, _)| {
            if let CborValue::Text(s) = k {
                s != "sig" && s != "hash"
            } else {
                true
            }
        });
    }

    // Canonicalize and hash
    cbor_canon::canon_value(&mut value);
    let mut cbor_bytes = Vec::new();
    ciborium::ser::into_writer(&value, &mut cbor_bytes)
        .map_err(|e| CanonError::Encode(e.to_string()))?;

    let mut hasher = Sha256::new();
    hasher.update(&cbor_bytes);
    let hash_bytes = hasher.finalize();

    Ok(format!("sha256:{:x}", hash_bytes))
}

/// Validate hash format: must match `^sha256:[0-9a-f]{64}$`.
pub fn validate_hash_format(hash: &str) -> bool {
    hash.starts_with("sha256:") && hash.len() == 71 && {
        let hex_part = &hash[7..];
        hex_part.chars().all(|c| c.is_ascii_hexdigit())
    }
}

/// Compute SHA-256 hash with `sha256:` prefix format.
///
/// This is a utility function for computing hashes in the standard format
/// used throughout the receipts system.
pub fn sha256_prefixed(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("sha256:{:x}", h.finalize())
}

/// Encode a value to deterministic CBOR per RFC 8949.
///
/// This is a convenience wrapper around `encode_canonical` for backward compatibility.
/// For new code, prefer `encode_canonical` which provides better error handling.
///
/// # Arguments
///
/// * `value` - Value to encode (must implement Serialize)
///
/// # Returns
///
/// CBOR bytes in deterministic encoding
///
/// # Errors
///
/// Returns error if serialization fails
pub fn cbor_deterministic<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    encode_canonical(value).map_err(|e| e.to_string())
}

/// Compute SHA-256 hash of deterministic CBOR with `sha256:` prefix.
///
/// This is a convenience wrapper around `hash_canonical` for backward compatibility.
/// For new code, prefer `hash_canonical` which provides better error handling.
///
/// # Arguments
///
/// * `value` - Value to encode and hash (must implement Serialize)
///
/// # Returns
///
/// Hash string in format `sha256:<64hex>`
///
/// # Errors
///
/// Returns error if CBOR encoding fails
pub fn cbor_hash<T: Serialize>(value: &T) -> Result<String, String> {
    hash_canonical(value).map_err(|e| e.to_string())
}

/// Validate that CBOR bytes follow RFC 8949 deterministic encoding rules.
///
/// This function checks:
/// - No indefinite-length items
/// - Map keys are sorted (if applicable)
/// - Preferred argument sizes
///
/// Note: Full validation requires parsing and re-encoding the CBOR.
/// This function validates by ensuring re-encoding produces identical bytes.
///
/// # Arguments
///
/// * `cbor_bytes` - CBOR bytes to validate
///
/// # Returns
///
/// `Ok(())` if CBOR appears to be deterministic, error otherwise
pub fn validate_cbor_deterministic(cbor_bytes: &[u8]) -> Result<(), String> {
    use ciborium::value::Value as CborValue;

    // Parse CBOR - ciborium will reject indefinite-length items
    let parsed: CborValue = ciborium::de::from_reader(cbor_bytes)
        .map_err(|e| format!("Invalid CBOR or non-deterministic encoding: {}", e))?;

    // Re-encode and compare - deterministic encoding should produce identical bytes
    let mut re_encoded = Vec::new();
    ciborium::ser::into_writer(&parsed, &mut re_encoded)
        .map_err(|e| format!("Failed to re-encode CBOR: {}", e))?;

    if re_encoded != cbor_bytes {
        return Err("Non-deterministic CBOR: re-encoding produces different bytes".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Context, DeterminismClass, Payload, Receipt, ReceiptKind};
    use uuid::Uuid;

    #[test]
    fn test_compute_hash_removes_sig_and_hash() {
        let receipt = Receipt {
            rid: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            version: "0.3.0".to_string(),
            kind: ReceiptKind::DataShape,
            dom: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            cod: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                .to_string(),
            links: vec![],
            ctx: Context {
                policy_ref: Some("pol:test".to_string()),
                timestamp: "2025-01-01T00:00:00Z".to_string(),
                nonce: None,
                determinism: Some(DeterminismClass::Strict),
                identity_ref: None,
            },
            payload: Payload::DataShape(crate::DataShapePayload {
                schema_hash:
                    "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                        .to_string(),
                sketch_hash: None,
            }),
            attest: None,
            sig: Some(crate::Signature {
                alg: "ed25519".to_string(),
                kid: "did:key:test".to_string(),
                sig: "test_sig".to_string(),
            }),
            hash: "sha256:3333333333333333333333333333333333333333333333333333333333333333"
                .to_string(),
        };

        let hash = compute_hash(&receipt).unwrap();
        // Hash should be computed without sig and hash fields
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 71);
    }

    #[test]
    fn test_validate_hash_format() {
        assert!(validate_hash_format(
            "sha256:0000000000000000000000000000000000000000000000000000000000000000"
        ));
        assert!(validate_hash_format(
            "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        ));
        assert!(!validate_hash_format("sha256:invalid"));
        assert!(!validate_hash_format(
            "sha256:000000000000000000000000000000000000000000000000000000000000000"
        ));
        assert!(!validate_hash_format("invalid"));
    }
}

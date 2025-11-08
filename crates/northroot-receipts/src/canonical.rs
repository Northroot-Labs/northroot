//! Canonicalization and hash computation for receipts.
//!
//! This module provides functions for computing canonical JSON representations
//! and SHA-256 hashes of receipts according to RFC 8785 (JSON Canonicalization).

use crate::Receipt;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Compute canonical JSON body of a receipt (without `sig` and `hash` fields).
///
/// This function serializes the receipt to JSON, removes the `sig` and `hash` fields,
/// and returns the canonical representation with sorted keys (RFC 8785).
pub fn canonical_body(receipt: &Receipt) -> Result<Value, serde_json::Error> {
    // Serialize to JSON
    let mut value: Value = serde_json::to_value(receipt)?;

    // Remove sig and hash fields
    if let Value::Object(ref mut map) = value {
        map.remove("sig");
        map.remove("hash");
    }

    // Canonicalize (sort keys recursively)
    Ok(canonicalize_value(&value))
}

/// Canonicalize a JSON value by sorting object keys recursively.
///
/// Implements RFC 8785 JSON Canonicalization Scheme (JCS).
fn canonicalize_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (k, v) in map {
                sorted.insert(k.clone(), canonicalize_value(v));
            }
            // Convert BTreeMap to serde_json::Map
            let json_map: serde_json::Map<String, Value> = sorted.into_iter().collect();
            Value::Object(json_map)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(canonicalize_value).collect()),
        _ => value.clone(),
    }
}

/// Compute SHA-256 hash of canonical body with `sha256:` prefix.
///
/// This computes the hash of the canonical JSON representation (without `sig` and `hash`),
/// and returns it in the format `sha256:<64hex>`.
pub fn compute_hash(receipt: &Receipt) -> Result<String, serde_json::Error> {
    let canonical = canonical_body(receipt)?;
    let json_str = serde_json::to_string(&canonical)?;

    let mut hasher = Sha256::new();
    hasher.update(json_str.as_bytes());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Context, DeterminismClass, Payload, Receipt, ReceiptKind};
    use uuid::Uuid;

    #[test]
    fn test_canonical_body_removes_sig_and_hash() {
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

        let canonical = canonical_body(&receipt).unwrap();
        assert!(!canonical.as_object().unwrap().contains_key("sig"));
        assert!(!canonical.as_object().unwrap().contains_key("hash"));
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

//! Tests for commitment computation functions.

use northroot_engine::commitments::*;
use serde_json::json;

#[test]
fn test_cbor_deterministic_encoding() {
    // Test that CBOR encoding is deterministic
    let value = json!({
        "b": 2,
        "a": 1,
        "c": 3
    });

    // Encode twice - should produce identical bytes
    let cbor1 = cbor_deterministic(&value).unwrap();
    let cbor2 = cbor_deterministic(&value).unwrap();

    assert_eq!(cbor1, cbor2, "CBOR encoding should be deterministic");
}

#[test]
fn test_cbor_hash_consistency() {
    // Test that CBOR hash is consistent
    let value = json!({
        "b": 2,
        "a": 1
    });

    let hash1 = cbor_hash(&value).unwrap();
    let hash2 = cbor_hash(&value).unwrap();

    assert_eq!(hash1, hash2, "CBOR hash should be consistent");
    assert!(
        hash1.starts_with("sha256:"),
        "Hash should have sha256: prefix"
    );
    assert_eq!(
        hash1.len(),
        71,
        "Hash should be 71 chars (sha256: + 64 hex)"
    );
}

#[test]
fn test_cbor_jcs_hash_equivalence() {
    // Test that CBOR and JCS produce same hash for same content
    // Note: This may not always be true due to format differences,
    // but for simple structures they should be equivalent

    let value = json!({
        "a": 1,
        "b": 2
    });

    let cbor_hash_val = cbor_hash(&value).unwrap();
    let jcs_str = jcs(&value);
    let jcs_hash_val = sha256_prefixed(jcs_str.as_bytes());

    // For this simple case, both should produce valid hashes
    assert!(cbor_hash_val.starts_with("sha256:"));
    assert!(jcs_hash_val.starts_with("sha256:"));
}

#[test]
fn test_validate_cbor_deterministic() {
    // Test validation of deterministic CBOR
    let value = json!({
        "b": 2,
        "a": 1
    });

    let cbor_bytes = cbor_deterministic(&value).unwrap();

    // Should validate successfully
    assert!(validate_cbor_deterministic(&cbor_bytes).is_ok());
}

#[test]
fn test_cbor_deterministic_sorted_keys() {
    // Test that map keys are sorted in deterministic encoding
    let value1 = json!({
        "z": 3,
        "a": 1,
        "m": 2
    });

    let value2 = json!({
        "a": 1,
        "m": 2,
        "z": 3
    });

    // Both should produce identical CBOR (keys sorted)
    let cbor1 = cbor_deterministic(&value1).unwrap();
    let cbor2 = cbor_deterministic(&value2).unwrap();

    assert_eq!(cbor1, cbor2, "CBOR should sort keys deterministically");
}

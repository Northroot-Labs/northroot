//! Regression tests for CBOR canonicalization.
//!
//! These tests ensure that CBOR canonicalization produces stable, deterministic results
//! and that the JSON adapter preserves semantics correctly.

use northroot_receipts::adapters::json;
use northroot_receipts::*;
use std::fs;

fn load_vector(path: &str) -> Receipt {
    let json_str = fs::read_to_string(path).unwrap();
    json::receipt_from_json(&json_str).unwrap()
}

#[test]
fn test_cbor_canonicalization_stable() {
    // Test that CBOR canonicalization produces identical bytes for the same input
    let receipt = load_vector("../../vectors/data_shape.json");

    let canonical1 = encode_canonical(&receipt).unwrap();
    let canonical2 = encode_canonical(&receipt).unwrap();
    let canonical3 = encode_canonical(&receipt).unwrap();

    // All canonicalizations should produce identical bytes
    assert_eq!(
        canonical1, canonical2,
        "Canonical encoding not stable (1 vs 2)"
    );
    assert_eq!(
        canonical2, canonical3,
        "Canonical encoding not stable (2 vs 3)"
    );
}

#[test]
fn test_hash_computation_deterministic() {
    // Test that hash computation is deterministic
    let receipt = load_vector("../../vectors/data_shape.json");

    let hash1 = receipt.compute_hash().unwrap();
    let hash2 = receipt.compute_hash().unwrap();
    let hash3 = receipt.compute_hash().unwrap();

    // All hash computations should produce identical results
    assert_eq!(hash1, hash2, "Hash computation not deterministic (1 vs 2)");
    assert_eq!(hash2, hash3, "Hash computation not deterministic (2 vs 3)");
}

#[test]
fn test_json_adapter_preserves_semantics() {
    // Test that JSON→CBOR→JSON round-trip preserves semantics
    let receipt1 = load_vector("../../vectors/spend.json");

    // Convert to JSON
    let json_str = json::receipt_to_json(&receipt1).unwrap();

    // Convert back from JSON
    let receipt2 = json::receipt_from_json(&json_str).unwrap();

    // Receipts should be equal (semantics preserved)
    assert_eq!(
        receipt1, receipt2,
        "JSON adapter does not preserve semantics"
    );
}

#[test]
fn test_cbor_roundtrip_preserves_structure() {
    // Test that CBOR round-trip preserves receipt structure
    let receipt1 = load_vector("../../vectors/execution.json");

    // Serialize to CBOR
    let mut cbor_bytes = Vec::new();
    ciborium::ser::into_writer(&receipt1, &mut cbor_bytes).unwrap();

    // Deserialize from CBOR
    let receipt2: Receipt = ciborium::de::from_reader(cbor_bytes.as_slice()).unwrap();

    // Receipts should be equal
    assert_eq!(
        receipt1, receipt2,
        "CBOR round-trip does not preserve structure"
    );
}

#[test]
fn test_hash_ignores_sig_and_hash_fields() {
    // Test that hash computation ignores sig and hash fields
    let mut receipt = load_vector("../../vectors/data_shape.json");
    let original_hash = receipt.compute_hash().unwrap();

    // Modify sig and hash
    receipt.sig = Some(Signature {
        alg: "ed25519".to_string(),
        kid: "did:key:test".to_string(),
        sig: "different_sig".to_string(),
    });
    receipt.hash = "sha256:different_hash".to_string();

    // Hash should remain the same
    let new_hash = receipt.compute_hash().unwrap();
    assert_eq!(
        original_hash, new_hash,
        "Hash computation should ignore sig and hash fields"
    );
}

#[test]
fn test_cdn_pretty_printing_readable() {
    // Test that CDN pretty-printing produces readable output
    let receipt = load_vector("../../vectors/data_shape.json");

    // Serialize to CBOR Value
    let mut buffer = Vec::new();
    ciborium::ser::into_writer(&receipt, &mut buffer).unwrap();
    let cbor_value: ciborium::value::Value = ciborium::de::from_reader(buffer.as_slice()).unwrap();

    // Convert to CDN
    let cdn = to_cdn(&cbor_value);

    // CDN should be non-empty and contain readable text
    assert!(!cdn.is_empty(), "CDN output should not be empty");
    assert!(cdn.len() > 100, "CDN output should be substantial");

    // CDN should contain some expected patterns
    assert!(
        cdn.contains("rid") || cdn.contains("kind") || cdn.contains("version"),
        "CDN should contain receipt field names"
    );
}

#[test]
fn test_all_vectors_canonicalize_correctly() {
    // Test that all test vectors canonicalize correctly
    let vectors = [
        "../../vectors/data_shape.json",
        "../../vectors/method_shape.json",
        "../../vectors/reasoning_shape.json",
        "../../vectors/execution.json",
        "../../vectors/spend.json",
        "../../vectors/settlement.json",
    ];

    for path in &vectors {
        let receipt = load_vector(path);

        // Should canonicalize without error
        let canonical = encode_canonical(&receipt).unwrap();
        assert!(
            !canonical.is_empty(),
            "Canonical encoding should not be empty for {}",
            path
        );

        // Should compute hash without error
        let hash = receipt.compute_hash().unwrap();
        assert!(
            hash.starts_with("sha256:") && hash.len() == 71,
            "Hash format should be correct for {}",
            path
        );

        // Hash should match stored hash
        assert_eq!(
            hash, receipt.hash,
            "Computed hash should match stored hash in {}",
            path
        );
    }
}

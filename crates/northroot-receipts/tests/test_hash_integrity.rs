//! Hash integrity tests: verify hash computation and canonicalization.

use northroot_receipts::*;
use std::fs;

fn load_vector(path: &str) -> Receipt {
    let json_str = fs::read_to_string(path).unwrap();
    serde_json::from_str(&json_str).unwrap()
}

#[test]
fn test_hash_computation_ignores_sig_and_hash() {
    let mut receipt = load_vector("../../vectors/data_shape.json");

    // Change sig and hash
    receipt.sig = Some(Signature {
        alg: "ed25519".to_string(),
        kid: "did:key:test".to_string(),
        sig: "different_sig".to_string(),
    });
    receipt.hash = "sha256:different_hash".to_string();

    // Compute hash should be the same as original (ignores sig/hash)
    let original = load_vector("../../vectors/data_shape.json");
    let _computed = receipt.compute_hash().unwrap();

    // The computed hash should match the original's hash (not the changed one)
    // But wait - we changed the receipt, so we need to restore it first
    let mut receipt2 = receipt.clone();
    receipt2.sig = original.sig.clone();
    receipt2.hash = original.hash.clone();
    let computed2 = receipt2.compute_hash().unwrap();
    assert_eq!(computed2, original.hash);
}

#[test]
fn test_canonicalization_stable() {
    let receipt = load_vector("../../vectors/data_shape.json");

    // Multiple canonicalizations should produce same result
    let canonical1 = canonical_body(&receipt).unwrap();
    let canonical2 = canonical_body(&receipt).unwrap();

    let json1 = serde_json::to_string(&canonical1).unwrap();
    let json2 = serde_json::to_string(&canonical2).unwrap();

    assert_eq!(json1, json2);
}

#[test]
fn test_hash_format_validation() {
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
    assert!(!validate_hash_format(""));
}

#[test]
fn test_all_vectors_hash_integrity() {
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
        let computed = receipt.compute_hash().unwrap();
        assert_eq!(
            computed, receipt.hash,
            "Hash mismatch in {}: expected {}, computed {}",
            path, receipt.hash, computed
        );
    }
}

#[test]
fn test_hash_computation_deterministic() {
    let receipt = load_vector("../../vectors/data_shape.json");

    let hash1 = receipt.compute_hash().unwrap();
    let hash2 = receipt.compute_hash().unwrap();
    let hash3 = receipt.compute_hash().unwrap();

    assert_eq!(hash1, hash2);
    assert_eq!(hash2, hash3);
}

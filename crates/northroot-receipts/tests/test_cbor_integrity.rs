//! CBOR integrity tests for receipts crate.
//!
//! This test ensures that receipt structures can be encoded to CBOR
//! deterministically and that CBOR encoding doesn't break existing receipt validation.

use northroot_receipts::adapters::json;
use northroot_receipts::*;
use std::fs;

fn load_vector(path: &str) -> Result<Receipt, Box<dyn std::error::Error>> {
    let json_str = fs::read_to_string(path)?;
    // Test vectors now have CBOR-based hashes
    let receipt = json::receipt_from_json(&json_str)?;
    Ok(receipt)
}

#[test]
fn test_receipt_cbor_encoding() {
    // Test that receipts can be encoded to CBOR deterministically
    let vectors = [
        "../../vectors/data_shape.json",
        "../../vectors/method_shape.json",
        "../../vectors/reasoning_shape.json",
        "../../vectors/execution.json",
        "../../vectors/spend.json",
        "../../vectors/settlement.json",
    ];

    for path in &vectors {
        let receipt =
            load_vector(path).unwrap_or_else(|e| panic!("Failed to load {}: {}", path, e));

        // Encode receipt payload to CBOR
        let cbor_bytes = cbor_deterministic(&receipt.payload)
            .unwrap_or_else(|e| panic!("Failed to encode {} to CBOR: {}", path, e));

        // Verify CBOR is deterministic (re-encode should produce same bytes)
        let cbor_bytes2 = cbor_deterministic(&receipt.payload)
            .unwrap_or_else(|e| panic!("Failed to re-encode {} to CBOR: {}", path, e));

        assert_eq!(
            cbor_bytes, cbor_bytes2,
            "CBOR encoding not deterministic for {}",
            path
        );

        // Verify CBOR validation passes
        assert!(
            validate_cbor_deterministic(&cbor_bytes).is_ok(),
            "CBOR validation failed for {}",
            path
        );

        // Verify hash computation works
        let cbor_hash_val = cbor_hash(&receipt.payload)
            .unwrap_or_else(|e| panic!("Failed to compute CBOR hash for {}: {}", path, e));

        assert!(
            cbor_hash_val.starts_with("sha256:"),
            "CBOR hash should have sha256: prefix for {}",
            path
        );
        assert_eq!(
            cbor_hash_val.len(),
            71,
            "CBOR hash should be 71 chars (sha256: + 64 hex) for {}",
            path
        );
    }
}

#[test]
fn test_receipt_cbor_does_not_break_json_validation() {
    // Ensure CBOR encoding doesn't affect JSON-based receipt validation
    let receipt = load_vector("../../vectors/spend.json")
        .unwrap_or_else(|e| panic!("Failed to load spend.json: {}", e));

    // Original JSON validation should still work
    receipt.validate().unwrap();

    // CBOR encoding should not affect receipt structure
    let _cbor_bytes = cbor_deterministic(&receipt.payload).unwrap();
    let _cbor_hash = cbor_hash(&receipt.payload).unwrap();

    // Receipt should still validate after CBOR operations
    receipt.validate().unwrap();

    // Hash computation should still work (now using CBOR canonicalization)
    let computed_hash = receipt.compute_hash().unwrap();
    assert_eq!(
        computed_hash, receipt.hash,
        "Hash should match computed value"
    );
}

#[test]
fn test_spend_receipt_cbor_with_alpha() {
    // Test that spend receipts with alpha justification encode correctly to CBOR
    let receipt = load_vector("../../vectors/spend.json")
        .unwrap_or_else(|e| panic!("Failed to load spend.json: {}", e));

    // Extract alpha (should be 0.9 from spend.json)
    let alpha = receipt.alpha();
    assert_eq!(alpha, Some(0.9), "Spend receipt should have alpha=0.9");

    // Encode to CBOR
    let cbor_bytes = cbor_deterministic(&receipt.payload).unwrap();
    let cbor_hash_val = cbor_hash(&receipt.payload).unwrap();

    // Verify CBOR is valid
    assert!(validate_cbor_deterministic(&cbor_bytes).is_ok());

    // Verify hash format
    assert!(cbor_hash_val.starts_with("sha256:"));
    assert_eq!(cbor_hash_val.len(), 71);
}

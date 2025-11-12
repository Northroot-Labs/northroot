//! Composition vector roundtrip tests.
//!
//! This test verifies that composition vectors can be loaded, validated,
//! and used in composition operations correctly.

use northroot_engine::*;
use northroot_receipts::{adapters::json, Receipt};
use std::fs;

fn load_vector(path: &str) -> Result<Vec<Receipt>, Box<dyn std::error::Error>> {
    let json_str = fs::read_to_string(path)?;
    // Parse JSON array of receipts
    let json_value: serde_json::Value = serde_json::from_str(&json_str)?;
    let mut receipts = Vec::new();
    if let serde_json::Value::Array(arr) = json_value {
        for item in arr {
            let json_str = serde_json::to_string(&item)?;
            // Test vectors now have CBOR-based hashes
            let receipt = json::receipt_from_json(&json_str)?;
            receipts.push(receipt);
        }
    }
    Ok(receipts)
}

#[test]
fn test_valid_chain_roundtrip() {
    // Load valid chain
    let chain = load_vector("../../vectors/engine/composition_chain_valid.json")
        .unwrap_or_else(|e| panic!("Failed to load valid chain: {}", e));

    // Verify it has all 6 receipt kinds
    assert_eq!(chain.len(), 6, "Valid chain must have all 6 receipt kinds");

    // Verify build_sequential_chain accepts it
    let built = build_sequential_chain(chain.clone())
        .unwrap_or_else(|e| panic!("Valid chain should pass build_sequential_chain: {}", e));

    assert_eq!(built.len(), 6, "Built chain should have all 6 receipts");

    // Verify validate_sequential passes
    assert!(
        validate_sequential(&built).is_ok(),
        "Built chain should pass validate_sequential"
    );

    // Verify all receipts are valid
    for (idx, receipt) in built.iter().enumerate() {
        receipt
            .validate()
            .unwrap_or_else(|e| panic!("Receipt {} in built chain failed validation: {}", idx, e));
    }
}

#[test]
fn test_invalid_chain_roundtrip() {
    // Load invalid chain
    let chain = load_vector("../../vectors/engine/composition_chain_invalid.json")
        .unwrap_or_else(|e| panic!("Failed to load invalid chain: {}", e));

    // Verify build_sequential_chain rejects it
    let result = build_sequential_chain(chain);
    assert!(
        result.is_err(),
        "Invalid chain should be rejected by build_sequential_chain"
    );

    // Verify it's a SequentialMismatch error
    match result.unwrap_err() {
        CompositionError::SequentialMismatch { receipt_index, .. } => {
            assert_eq!(receipt_index, 1, "Mismatch should be at index 1");
        }
        _ => panic!("Expected SequentialMismatch error"),
    }
}

#[test]
fn test_tensor_composition_roundtrip() {
    use serde_json::Value;
    use std::fs;

    // Load tensor composition vector
    let json_str = fs::read_to_string("../../vectors/engine/tensor_composition.json")
        .unwrap_or_else(|e| panic!("Failed to load tensor composition: {}", e));
    let tensor_data: Value = serde_json::from_str(&json_str)
        .unwrap_or_else(|e| panic!("Failed to parse tensor composition: {}", e));

    let child_hashes: Vec<String> = tensor_data["child_receipt_hashes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    let expected_root = tensor_data["tensor_root"].as_str().unwrap();

    // Compute tensor root
    let computed_root = compute_tensor_root(&child_hashes);

    // Verify it matches expected
    assert_eq!(
        computed_root, expected_root,
        "Computed tensor root should match expected"
    );

    // Verify order independence
    let mut reversed = child_hashes.clone();
    reversed.reverse();
    let reversed_root = compute_tensor_root(&reversed);
    assert_eq!(
        computed_root, reversed_root,
        "Tensor root should be order-independent"
    );
}

#[test]
fn test_valid_chain_all_kinds() {
    // Verify valid chain contains all 6 receipt kinds from proof algebra spec
    let chain = load_vector("../../vectors/engine/composition_chain_valid.json")
        .unwrap_or_else(|e| panic!("Failed to load valid chain: {}", e));

    use northroot_receipts::ReceiptKind;

    let expected_kinds = vec![
        ReceiptKind::DataShape,
        ReceiptKind::MethodShape,
        ReceiptKind::ReasoningShape,
        ReceiptKind::Execution,
        ReceiptKind::Spend,
        ReceiptKind::Settlement,
    ];

    assert_eq!(
        chain.len(),
        expected_kinds.len(),
        "Chain must have all 6 receipt kinds"
    );

    for (idx, (receipt, expected_kind)) in chain.iter().zip(expected_kinds.iter()).enumerate() {
        assert_eq!(
            receipt.kind, *expected_kind,
            "Receipt at index {} should be {:?}, got {:?}",
            idx, expected_kind, receipt.kind
        );
    }
}

#[test]
fn test_chain_composition_validation() {
    // Test that validate_sequential correctly identifies valid and invalid chains
    let valid_chain = load_vector("../../vectors/engine/composition_chain_valid.json")
        .unwrap_or_else(|e| panic!("Failed to load valid chain: {}", e));

    let invalid_chain = load_vector("../../vectors/engine/composition_chain_invalid.json")
        .unwrap_or_else(|e| panic!("Failed to load invalid chain: {}", e));

    // Valid chain should pass
    assert!(
        validate_sequential(&valid_chain).is_ok(),
        "Valid chain should pass validate_sequential"
    );

    // Invalid chain should fail
    assert!(
        validate_sequential(&invalid_chain).is_err(),
        "Invalid chain should fail validate_sequential"
    );
}

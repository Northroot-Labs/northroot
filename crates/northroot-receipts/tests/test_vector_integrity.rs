//! Test vector integrity: verify existing vectors have correct hashes and structure.

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
fn test_data_shape_vector_integrity() {
    let receipt = load_vector("../../vectors/data_shape.json").unwrap();

    // Verify hash integrity
    let computed_hash = receipt.compute_hash().unwrap();
    assert_eq!(
        computed_hash, receipt.hash,
        "Hash mismatch in data_shape.json"
    );

    // Verify validation
    receipt.validate().unwrap();
}

#[test]
fn test_method_shape_vector_integrity() {
    let receipt = load_vector("../../vectors/method_shape.json").unwrap();

    let computed_hash = receipt.compute_hash().unwrap();
    assert_eq!(
        computed_hash, receipt.hash,
        "Hash mismatch in method_shape.json"
    );

    receipt.validate().unwrap();
}

#[test]
fn test_reasoning_shape_vector_integrity() {
    let receipt = load_vector("../../vectors/reasoning_shape.json").unwrap();

    let computed_hash = receipt.compute_hash().unwrap();
    assert_eq!(
        computed_hash, receipt.hash,
        "Hash mismatch in reasoning_shape.json"
    );

    receipt.validate().unwrap();
}

#[test]
fn test_execution_vector_integrity() {
    let receipt = load_vector("../../vectors/execution.json").unwrap();

    let computed_hash = receipt.compute_hash().unwrap();
    assert_eq!(
        computed_hash, receipt.hash,
        "Hash mismatch in execution.json"
    );

    receipt.validate().unwrap();
}

#[test]
fn test_spend_vector_integrity() {
    let receipt = load_vector("../../vectors/spend.json").unwrap();

    let computed_hash = receipt.compute_hash().unwrap();
    assert_eq!(computed_hash, receipt.hash, "Hash mismatch in spend.json");

    receipt.validate().unwrap();
}

#[test]
fn test_settlement_vector_integrity() {
    let receipt = load_vector("../../vectors/settlement.json").unwrap();

    let computed_hash = receipt.compute_hash().unwrap();
    assert_eq!(
        computed_hash, receipt.hash,
        "Hash mismatch in settlement.json"
    );

    receipt.validate().unwrap();
}

#[test]
fn test_all_vectors_have_valid_hash_format() {
    let vectors = [
        "../../vectors/data_shape.json",
        "../../vectors/method_shape.json",
        "../../vectors/reasoning_shape.json",
        "../../vectors/execution.json",
        "../../vectors/spend.json",
        "../../vectors/settlement.json",
    ];

    for path in &vectors {
        let receipt = load_vector(path).unwrap();

        // Check hash format
        assert!(
            validate_hash_format(&receipt.hash),
            "Invalid hash format in {}: {}",
            path,
            receipt.hash
        );

        // Check dom and cod formats
        assert!(
            validate_hash_format(&receipt.dom),
            "Invalid dom format in {}: {}",
            path,
            receipt.dom
        );
        assert!(
            validate_hash_format(&receipt.cod),
            "Invalid cod format in {}: {}",
            path,
            receipt.cod
        );
    }
}

#[test]
fn test_sequential_chain_composition() {
    let data_shape = load_vector("../../vectors/data_shape.json").unwrap();
    let method_shape = load_vector("../../vectors/method_shape.json").unwrap();
    let reasoning_shape = load_vector("../../vectors/reasoning_shape.json").unwrap();
    let execution = load_vector("../../vectors/execution.json").unwrap();
    let spend = load_vector("../../vectors/spend.json").unwrap();
    let settlement = load_vector("../../vectors/settlement.json").unwrap();

    let chain = vec![
        data_shape,
        method_shape,
        reasoning_shape,
        execution,
        spend,
        settlement,
    ];

    // Verify composition
    validate_composition(&chain).unwrap();
}

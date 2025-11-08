//! Round-trip tests: serialize → deserialize → compare.

use northroot_receipts::*;
use std::fs;

fn load_vector(path: &str) -> Receipt {
    let json_str = fs::read_to_string(path).unwrap();
    serde_json::from_str(&json_str).unwrap()
}

#[test]
fn test_data_shape_roundtrip() {
    let receipt1 = load_vector("../../vectors/data_shape.json");
    let json = serde_json::to_string(&receipt1).unwrap();
    let receipt2: Receipt = serde_json::from_str(&json).unwrap();

    assert_eq!(receipt1, receipt2);
}

#[test]
fn test_method_shape_roundtrip() {
    let receipt1 = load_vector("../../vectors/method_shape.json");
    let json = serde_json::to_string(&receipt1).unwrap();
    let receipt2: Receipt = serde_json::from_str(&json).unwrap();

    assert_eq!(receipt1, receipt2);
}

#[test]
fn test_reasoning_shape_roundtrip() {
    let receipt1 = load_vector("../../vectors/reasoning_shape.json");
    let json = serde_json::to_string(&receipt1).unwrap();
    let receipt2: Receipt = serde_json::from_str(&json).unwrap();

    assert_eq!(receipt1, receipt2);
}

#[test]
fn test_execution_roundtrip() {
    let receipt1 = load_vector("../../vectors/execution.json");
    let json = serde_json::to_string(&receipt1).unwrap();
    let receipt2: Receipt = serde_json::from_str(&json).unwrap();

    assert_eq!(receipt1, receipt2);
}

#[test]
fn test_spend_roundtrip() {
    let receipt1 = load_vector("../../vectors/spend.json");
    let json = serde_json::to_string(&receipt1).unwrap();
    let receipt2: Receipt = serde_json::from_str(&json).unwrap();

    assert_eq!(receipt1, receipt2);
}

#[test]
fn test_settlement_roundtrip() {
    let receipt1 = load_vector("../../vectors/settlement.json");
    let json = serde_json::to_string(&receipt1).unwrap();
    let receipt2: Receipt = serde_json::from_str(&json).unwrap();

    assert_eq!(receipt1, receipt2);
}

#[test]
fn test_multiple_roundtrips() {
    let receipt1 = load_vector("../../vectors/data_shape.json");
    let mut receipt = receipt1;

    // Multiple round-trips
    for _ in 0..5 {
        let json = serde_json::to_string(&receipt).unwrap();
        receipt = serde_json::from_str(&json).unwrap();
    }

    // Should still validate
    receipt.validate().unwrap();
}

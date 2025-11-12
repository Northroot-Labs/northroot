//! Round-trip tests: serialize → deserialize → compare.
//!
//! Tests use JSON adapters for loading test vectors (human-readable),
//! but internally use CBOR for all operations.

use northroot_receipts::adapters::json;
use northroot_receipts::*;
use std::fs;

fn load_vector(path: &str) -> Receipt {
    let json_str = fs::read_to_string(path).unwrap();
    json::receipt_from_json(&json_str).unwrap()
}

#[test]
fn test_data_shape_roundtrip() {
    let receipt1 = load_vector("../../vectors/data_shape.json");
    // Round-trip through CBOR (internal format)
    // Use regular CBOR serialization (not canonical) for round-trip test
    // Canonical encoding is for hashing/signing, not for storage/transmission
    let mut cbor_bytes = Vec::new();
    ciborium::ser::into_writer(&receipt1, &mut cbor_bytes).unwrap();
    let receipt2: Receipt = ciborium::de::from_reader(cbor_bytes.as_slice()).unwrap();

    assert_eq!(receipt1, receipt2);
}

#[test]
fn test_method_shape_roundtrip() {
    let receipt1 = load_vector("../../vectors/method_shape.json");
    let mut cbor_bytes = Vec::new();
    ciborium::ser::into_writer(&receipt1, &mut cbor_bytes).unwrap();
    let receipt2: Receipt = ciborium::de::from_reader(cbor_bytes.as_slice()).unwrap();
    assert_eq!(receipt1, receipt2);
}

#[test]
fn test_reasoning_shape_roundtrip() {
    let receipt1 = load_vector("../../vectors/reasoning_shape.json");
    let mut cbor_bytes = Vec::new();
    ciborium::ser::into_writer(&receipt1, &mut cbor_bytes).unwrap();
    let receipt2: Receipt = ciborium::de::from_reader(cbor_bytes.as_slice()).unwrap();
    assert_eq!(receipt1, receipt2);
}

#[test]
fn test_execution_roundtrip() {
    let receipt1 = load_vector("../../vectors/execution.json");
    let mut cbor_bytes = Vec::new();
    ciborium::ser::into_writer(&receipt1, &mut cbor_bytes).unwrap();
    let receipt2: Receipt = ciborium::de::from_reader(cbor_bytes.as_slice()).unwrap();
    assert_eq!(receipt1, receipt2);
}

#[test]
fn test_spend_roundtrip() {
    let receipt1 = load_vector("../../vectors/spend.json");
    let mut cbor_bytes = Vec::new();
    ciborium::ser::into_writer(&receipt1, &mut cbor_bytes).unwrap();
    let receipt2: Receipt = ciborium::de::from_reader(cbor_bytes.as_slice()).unwrap();
    assert_eq!(receipt1, receipt2);
}

#[test]
fn test_settlement_roundtrip() {
    let receipt1 = load_vector("../../vectors/settlement.json");
    let mut cbor_bytes = Vec::new();
    ciborium::ser::into_writer(&receipt1, &mut cbor_bytes).unwrap();
    let receipt2: Receipt = ciborium::de::from_reader(cbor_bytes.as_slice()).unwrap();
    assert_eq!(receipt1, receipt2);
}

#[test]
fn test_multiple_roundtrips() {
    let receipt1 = load_vector("../../vectors/data_shape.json");
    let mut receipt = receipt1;

    // Multiple round-trips through CBOR (internal format)
    for _ in 0..5 {
        let mut cbor_bytes = Vec::new();
        ciborium::ser::into_writer(&receipt, &mut cbor_bytes).unwrap();
        receipt = ciborium::de::from_reader(cbor_bytes.as_slice()).unwrap();
    }

    // Recompute hash after round-trips (hash is computed from canonical form)
    receipt.hash = receipt.compute_hash().unwrap();

    // Should still validate
    receipt.validate().unwrap();
}

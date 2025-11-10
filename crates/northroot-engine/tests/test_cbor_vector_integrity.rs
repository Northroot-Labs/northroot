//! CBOR golden vector integrity tests.
//!
//! This test validates CBOR golden vectors to ensure:
//! - CBOR encoding is deterministic
//! - CBOR hashes match expected values
//! - Re-encoding produces identical bytes
//! - No breaking changes to CBOR encoding

use northroot_engine::commitments::*;
use serde_json::Value;
use std::fs;

/// Load and parse CBOR golden vectors JSON file.
fn load_cbor_vectors() -> Result<Value, Box<dyn std::error::Error>> {
    let json_str = fs::read_to_string("../../vectors/engine/cbor_golden_vectors.json")?;
    let value: Value = serde_json::from_str(&json_str)?;
    Ok(value)
}

#[test]
fn test_cbor_golden_vectors_integrity() {
    let vectors =
        load_cbor_vectors().unwrap_or_else(|e| panic!("Failed to load CBOR golden vectors: {}", e));

    let vectors_array = vectors
        .get("vectors")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("Missing 'vectors' array in golden vectors file"));

    for vector in vectors_array {
        let name = vector
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unnamed");
        let json_value = vector
            .get("json")
            .unwrap_or_else(|| panic!("Missing 'json' field in vector: {}", name));
        let expected_hash = vector
            .get("expected_cbor_hash")
            .and_then(|h| h.as_str())
            .unwrap_or_else(|| panic!("Missing 'expected_cbor_hash' in vector: {}", name));

        // Compute CBOR hash
        let computed_hash = cbor_hash(json_value)
            .unwrap_or_else(|e| panic!("Failed to compute CBOR hash for {}: {}", name, e));

        // Verify hash matches expected
        assert_eq!(
            computed_hash, expected_hash,
            "CBOR hash mismatch for vector '{}': expected {}, computed {}",
            name, expected_hash, computed_hash
        );

        // Verify deterministic encoding: re-encode should produce same bytes
        let cbor_bytes1 = cbor_deterministic(json_value)
            .unwrap_or_else(|e| panic!("Failed to encode CBOR for {}: {}", name, e));
        let cbor_bytes2 = cbor_deterministic(json_value)
            .unwrap_or_else(|e| panic!("Failed to re-encode CBOR for {}: {}", name, e));

        assert_eq!(
            cbor_bytes1, cbor_bytes2,
            "CBOR encoding not deterministic for vector '{}'",
            name
        );

        // Verify validation passes
        assert!(
            validate_cbor_deterministic(&cbor_bytes1).is_ok(),
            "CBOR validation failed for vector '{}'",
            name
        );
    }
}

#[test]
fn test_cbor_deterministic_key_sorting() {
    // Test that keys are sorted deterministically
    let value1 = serde_json::json!({
        "z": 3,
        "a": 1,
        "m": 2
    });

    let value2 = serde_json::json!({
        "a": 1,
        "m": 2,
        "z": 3
    });

    // Both should produce identical CBOR (keys sorted)
    let cbor1 = cbor_deterministic(&value1).unwrap();
    let cbor2 = cbor_deterministic(&value2).unwrap();

    assert_eq!(
        cbor1, cbor2,
        "CBOR should sort keys deterministically regardless of input order"
    );

    // Hashes should match
    let hash1 = cbor_hash(&value1).unwrap();
    let hash2 = cbor_hash(&value2).unwrap();
    assert_eq!(hash1, hash2, "CBOR hashes should match for same content");
}

#[test]
fn test_cbor_round_trip_integrity() {
    // Test round-trip: encode → decode → re-encode should produce same bytes
    use ciborium::value::Value as CborValue;

    let test_cases = vec![
        serde_json::json!({"a": 1, "b": 2}),
        serde_json::json!([1, 2, 3]),
        serde_json::json!({"nested": {"inner": 42}}),
        serde_json::json!(null),
        serde_json::json!(true),
        serde_json::json!(false),
        serde_json::json!(42),
        serde_json::json!(3.14),
    ];

    for json_value in test_cases {
        // Encode to CBOR
        let cbor_bytes =
            cbor_deterministic(&json_value).unwrap_or_else(|e| panic!("Failed to encode: {}", e));

        // Decode from CBOR
        let decoded: CborValue = ciborium::de::from_reader(cbor_bytes.as_slice())
            .unwrap_or_else(|e| panic!("Failed to decode CBOR: {}", e));

        // Re-encode
        let mut re_encoded = Vec::new();
        ciborium::ser::into_writer(&decoded, &mut re_encoded)
            .unwrap_or_else(|e| panic!("Failed to re-encode CBOR: {}", e));

        // Should produce identical bytes
        assert_eq!(
            cbor_bytes, re_encoded,
            "Round-trip encoding failed for value: {:?}",
            json_value
        );
    }
}

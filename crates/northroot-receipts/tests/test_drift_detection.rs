//! Drift detection test: ensures canonicalization hasn't changed unexpectedly.
//!
//! This test compares computed hashes against a stored baseline to detect
//! any changes to canonicalization logic that would break compatibility.

use northroot_receipts::*;
use std::fs;

/// Baseline hashes for all test vectors.
///
/// These hashes are computed from the canonical JSON representation of receipts.
/// If canonicalization changes, these hashes will change, alerting us to potential
/// compatibility issues.
///
/// To update baselines after intentional canonicalization changes:
/// 1. Run `cargo test --test regenerate_vectors -- --ignored --nocapture`
/// 2. Run `cargo test --test test_vector_integrity` to get new hashes
/// 3. Update this constant with the new hashes
const BASELINE_HASHES: &[(&str, &str)] = &[
    (
        "data_shape.json",
        "sha256:e58858ad998397eaa1db5642bcca7c0132ef5e0b7c76c6307ad2900cb696bcd7",
    ),
    (
        "method_shape.json",
        "sha256:b26a128e7e1fa304a9d43d92c96f3d0fcb9381a91d8fc87d13c1813f42361208",
    ),
    (
        "reasoning_shape.json",
        "sha256:4318692a6fe02e195581592b4df2e1052152fc9b8a71987222d1b1fd9ce50b61",
    ),
    (
        "execution.json",
        "sha256:1c5c123f8d6fe6356acb3557d8feb51c84c9804b68473f1c6bdbf887b63f59bb",
    ),
    (
        "spend.json",
        "sha256:f48dae8f229a7cf63c9ef49c2d4d74053bce8bba6d52589669d9b254584baa3b",
    ),
    (
        "settlement.json",
        "sha256:74b7bda7a92b49ecb21b36d72d1c4436c78c409a3264b5502de11b9e66c6a0ab",
    ),
];

fn load_vector(path: &str) -> Result<Receipt, Box<dyn std::error::Error>> {
    let json_str = fs::read_to_string(path)?;
    let receipt: Receipt = serde_json::from_str(&json_str)?;
    Ok(receipt)
}

#[test]
fn test_vector_hash_baselines() {
    // Build baseline map
    let vectors_dir = "../../vectors";
    let mut mismatches: Vec<(&str, &str, String)> = Vec::new();

    for (filename, expected_hash) in BASELINE_HASHES {
        let path = format!("{}/{}", vectors_dir, filename);
        let receipt =
            load_vector(&path).unwrap_or_else(|e| panic!("Failed to load {}: {}", path, e));

        let computed_hash = receipt
            .compute_hash()
            .unwrap_or_else(|e| panic!("Failed to compute hash for {}: {}", path, e));

        if computed_hash != *expected_hash {
            mismatches.push((
                filename,
                expected_hash,
                computed_hash.clone(), // Clone to own the value
            ));
        }
    }

    if !mismatches.is_empty() {
        eprintln!("\n⚠️  Hash drift detected! Canonicalization may have changed.\n");
        eprintln!("This could indicate:");
        eprintln!("  1. Intentional canonicalization changes (update BASELINE_HASHES)");
        eprintln!("  2. Unintended changes to serialization logic");
        eprintln!("  3. Changes to receipt structure that affect canonicalization\n");

        for (filename, expected, computed) in &mismatches {
            eprintln!("  {}:", filename);
            eprintln!("    Expected: {}", expected);
            eprintln!("    Computed: {}", computed);
        }

        eprintln!("\nTo update baselines after intentional changes:");
        eprintln!("  1. Run: cargo test --test regenerate_vectors -- --ignored --nocapture");
        eprintln!("  2. Run: cargo test --test test_vector_integrity");
        eprintln!("  3. Update BASELINE_HASHES in test_drift_detection.rs with new hashes\n");

        panic!("Hash drift detected in {} vector(s)", mismatches.len());
    }
}

#[test]
fn test_all_vectors_compute_hashes_consistently() {
    // Test that hash computation is deterministic and consistent.
    //
    // This ensures that:
    // 1. Hash computation is deterministic (same input → same hash)
    // 2. Hash computation is consistent across multiple calls
    // 3. Canonicalization produces stable results

    let vectors_dir = "../../vectors";
    let vectors = [
        "data_shape.json",
        "method_shape.json",
        "reasoning_shape.json",
        "execution.json",
        "spend.json",
        "settlement.json",
    ];

    for filename in &vectors {
        let path = format!("{}/{}", vectors_dir, filename);
        let receipt =
            load_vector(&path).unwrap_or_else(|e| panic!("Failed to load {}: {}", path, e));

        // Compute hash multiple times - should be identical
        let hash1 = receipt.compute_hash().unwrap();
        let hash2 = receipt.compute_hash().unwrap();
        let hash3 = receipt.compute_hash().unwrap();

        assert_eq!(
            hash1, hash2,
            "Hash computation not deterministic for {}",
            filename
        );
        assert_eq!(
            hash2, hash3,
            "Hash computation inconsistent for {}",
            filename
        );

        // Verify hash matches stored hash in receipt
        assert_eq!(
            hash1, receipt.hash,
            "Computed hash doesn't match stored hash in {}",
            filename
        );
    }
}

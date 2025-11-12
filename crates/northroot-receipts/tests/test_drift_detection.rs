//! Drift detection test: ensures canonicalization hasn't changed unexpectedly.
//!
//! This test compares computed hashes against a stored baseline to detect
//! any changes to canonicalization logic that would break compatibility.

use northroot_receipts::adapters::json;
use northroot_receipts::*;
use std::collections::{BTreeSet, HashSet};
use std::fs;

/// Baseline hashes for all test vectors.
///
/// **NOTE**: These hashes are now computed from CBOR canonicalization (RFC 8949),
/// not JSON (JCS). All hashes changed during the CBOR migration.
///
/// To update baselines after intentional canonicalization changes:
/// 1. Run `cargo test --test regenerate_vectors -- --ignored --nocapture`
/// 2. Run `cargo test --test test_vector_integrity` to get new hashes
/// 3. Update this constant with the new hashes
///
/// **Updated**: These hashes are computed from CBOR canonicalization (RFC 8949),
/// not JSON (JCS). All hashes changed during the CBOR migration.
const BASELINE_HASHES: &[(&str, &str)] = &[
    (
        "data_shape.json",
        "sha256:ae536d81d66aa23403581860d9540188d86ed4b90ae2f0bcaadfa50ccb4d0065",
    ),
    (
        "method_shape.json",
        "sha256:af589a4e472fb7f1644d9a6f98224973c754a87a4232619c1e49384c8b3d5281",
    ),
    (
        "reasoning_shape.json",
        "sha256:62b76e51f24d101e92581981e3a9b58785320983585768aa2dc7c1eabd1eb598",
    ),
    (
        "execution.json",
        "sha256:fbea7214346e0c3c650e52557c9aa914bd4083e6bced1b2b55dd979a44c42a5f",
    ),
    (
        "spend.json",
        "sha256:088b62232e0b9500985329afff5465b0d0faff1a76aafc3388509a700343cbcb",
    ),
    (
        "settlement.json",
        "sha256:06f1ad9ca953337908b2f032b9722da75b548e1e6fb36027dbc9aefebf4d756e",
    ),
];

fn load_vector(path: &str) -> Result<Receipt, Box<dyn std::error::Error>> {
    let json_str = fs::read_to_string(path)?;
    // Test vectors now have CBOR-based hashes, no need to recompute
    let receipt = json::receipt_from_json(&json_str)?;
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

        // Verify hash matches stored hash in receipt (which was recomputed during load)
        assert_eq!(
            hash1, receipt.hash,
            "Computed hash doesn't match stored hash in {}",
            filename
        );
    }
}

#[test]
fn test_alpha_threshold_validation() {
    // Test that α threshold validation works correctly for spend receipts.
    //
    // This ensures that:
    // 1. Receipts with α >= threshold pass validation
    // 2. Receipts with α < threshold fail validation
    // 3. Receipts without α pass validation (not all receipts have reuse justification)

    use northroot_receipts::test_utils::generate_spend_receipt;

    // Create a spend receipt with α = 0.9
    let receipt = generate_spend_receipt("sha256:test_dom");

    // Test: α=0.9 should pass when threshold=0.8
    assert!(
        receipt.validate_alpha_threshold(0.8).is_ok(),
        "Receipt with α=0.9 should pass threshold=0.8"
    );

    // Test: α=0.9 should fail when threshold=0.95
    assert!(
        receipt.validate_alpha_threshold(0.95).is_err(),
        "Receipt with α=0.9 should fail threshold=0.95"
    );

    // Test: α=0.9 should pass when threshold=0.9 (equal)
    assert!(
        receipt.validate_alpha_threshold(0.9).is_ok(),
        "Receipt with α=0.9 should pass threshold=0.9 (equal)"
    );

    // Test: Extract α value
    assert_eq!(
        receipt.alpha(),
        Some(0.9),
        "Should extract α=0.9 from spend receipt"
    );
}

#[test]
fn test_alpha_extraction_non_spend_receipts() {
    // Test that non-spend receipts return None for α extraction

    use northroot_receipts::test_utils::generate_execution_receipt;

    let receipt = generate_execution_receipt("sha256:test_dom");

    // Non-spend receipts should return None
    assert_eq!(
        receipt.alpha(),
        None,
        "Non-spend receipts should return None for α"
    );

    // Validation should pass (no α to check)
    assert!(
        receipt.validate_alpha_threshold(0.8).is_ok(),
        "Non-spend receipts should pass α validation (no α to check)"
    );
}

#[test]
fn test_alpha_threshold_edge_cases() {
    // Test edge cases for α threshold validation

    use northroot_receipts::test_utils::generate_spend_receipt;

    let receipt = generate_spend_receipt("sha256:test_dom");

    // Test: Invalid threshold values should fail
    assert!(
        receipt.validate_alpha_threshold(-0.1).is_err(),
        "Negative threshold should fail validation"
    );

    assert!(
        receipt.validate_alpha_threshold(1.1).is_err(),
        "Threshold > 1.0 should fail validation"
    );

    // Test: Boundary values
    assert!(
        receipt.validate_alpha_threshold(0.0).is_ok(),
        "Threshold=0.0 should pass (any α >= 0)"
    );

    assert!(
        receipt.validate_alpha_threshold(1.0).is_err(),
        "Threshold=1.0 should fail for α=0.9"
    );
}

#[test]
fn test_cdf_metadata_in_execution_receipt() {
    // Test that CDF metadata can be stored in execution receipts
    use northroot_receipts::*;
    use uuid::Uuid;

    let cdf_metadata = vec![
        CdfMetadata {
            commit_version: 1,
            change_type: "insert".to_string(),
            commit_timestamp: "2025-11-08T12:00:00Z".to_string(),
        },
        CdfMetadata {
            commit_version: 2,
            change_type: "update".to_string(),
            commit_timestamp: "2025-11-08T12:01:00Z".to_string(),
        },
        CdfMetadata {
            commit_version: 3,
            change_type: "delete".to_string(),
            commit_timestamp: "2025-11-08T12:02:00Z".to_string(),
        },
    ];

    let receipt = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::Execution,
        dom: "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        cod: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        links: vec![],
        ctx: Context {
            policy_ref: Some("pol:test".to_string()),
            timestamp: "2025-11-08T12:00:00Z".to_string(),
            nonce: None,
            determinism: Some(DeterminismClass::Strict),
            identity_ref: None,
        },
        payload: Payload::Execution(ExecutionPayload {
            trace_id: "tr_cdf_test_001".to_string(),
            method_ref: MethodRef {
                method_id: "com.acme/cdf_scan".to_string(),
                version: "1.0.0".to_string(),
                method_shape_root:
                    "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                        .to_string(),
            },
            data_shape_hash:
                "sha256:3333333333333333333333333333333333333333333333333333333333333333"
                    .to_string(),
            span_commitments: vec![
                "sha256:4444444444444444444444444444444444444444444444444444444444444444"
                    .to_string(),
            ],
            roots: ExecutionRoots {
                trace_set_root:
                    "sha256:5555555555555555555555555555555555555555555555555555555555555555"
                        .to_string(),
                identity_root:
                    "sha256:6666666666666666666666666666666666666666666666666666666666666666"
                        .to_string(),
                trace_seq_root: None,
            },
            cdf_metadata: Some(cdf_metadata.clone()),
            pac: None,
            change_epoch: None,
            minhash_signature: None,
            hll_cardinality: None,
            chunk_manifest_hash: None,
            chunk_manifest_size_bytes: None,
            merkle_root: None,
            prev_execution_rid: None,
            output_digest: None,
            manifest_root: None,
            output_mime_type: None,
            output_size_bytes: None,
            input_locator_refs: None,
            output_locator_ref: None,
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };

    // Compute hash
    let hash = receipt.compute_hash().unwrap();
    let receipt = Receipt { hash, ..receipt };

    // Verify CDF metadata is present
    if let Payload::Execution(exec_payload) = &receipt.payload {
        assert!(
            exec_payload.cdf_metadata.is_some(),
            "CDF metadata should be present"
        );
        let cdf = exec_payload.cdf_metadata.as_ref().unwrap();
        assert_eq!(cdf.len(), 3, "Should have 3 CDF entries");
        assert_eq!(cdf[0].commit_version, 1);
        assert_eq!(cdf[0].change_type, "insert");
        assert_eq!(cdf[1].commit_version, 2);
        assert_eq!(cdf[1].change_type, "update");
        assert_eq!(cdf[2].commit_version, 3);
        assert_eq!(cdf[2].change_type, "delete");
    } else {
        panic!("Receipt should be Execution type");
    }

    // Test serialization/deserialization roundtrip
    let json_str = serde_json::to_string(&receipt).unwrap();
    let deserialized: Receipt = serde_json::from_str(&json_str).unwrap();

    if let Payload::Execution(exec_payload) = &deserialized.payload {
        assert!(
            exec_payload.cdf_metadata.is_some(),
            "CDF metadata should survive roundtrip"
        );
        let cdf = exec_payload.cdf_metadata.as_ref().unwrap();
        assert_eq!(cdf.len(), 3);
        assert_eq!(cdf[0].commit_version, 1);
    }
}

/// CDF drift detector: tracks expected vs actual CDF commit versions.
///
/// This struct helps identify missing CDF commit versions that indicate
/// partition-level drift requiring recomputation.
#[derive(Debug, Clone)]
pub struct CdfDriftDetector {
    /// Expected commit versions (sorted set)
    expected_versions: BTreeSet<i64>,
    /// Partition ID to commit version mapping (for identifying affected partitions)
    partition_versions: std::collections::HashMap<String, Vec<i64>>,
}

impl CdfDriftDetector {
    /// Create a new CDF drift detector with expected commit versions.
    pub fn new(expected_versions: Vec<i64>) -> Self {
        Self {
            expected_versions: expected_versions.into_iter().collect(),
            partition_versions: std::collections::HashMap::new(),
        }
    }

    /// Add partition-to-commit-version mapping.
    ///
    /// This allows the detector to identify which partitions are affected
    /// when commit versions are missing.
    pub fn with_partition_versions(
        mut self,
        partition_versions: std::collections::HashMap<String, Vec<i64>>,
    ) -> Self {
        self.partition_versions = partition_versions;
        self
    }

    /// Detect drift by comparing expected vs actual CDF commit versions.
    ///
    /// # Arguments
    ///
    /// * `actual_receipts` - Receipts containing CDF metadata to check
    ///
    /// # Returns
    ///
    /// Tuple of (missing_versions, affected_partitions)
    pub fn detect_cdf_drift(&self, actual_receipts: &[Receipt]) -> (Vec<i64>, Vec<String>) {
        // Collect all commit versions from actual receipts
        let mut actual_versions = BTreeSet::new();
        for receipt in actual_receipts {
            if let Payload::Execution(exec_payload) = &receipt.payload {
                if let Some(cdf_metadata) = &exec_payload.cdf_metadata {
                    for cdf in cdf_metadata {
                        actual_versions.insert(cdf.commit_version);
                    }
                }
            }
        }

        // Find missing versions
        let missing_versions: Vec<i64> = self
            .expected_versions
            .difference(&actual_versions)
            .cloned()
            .collect();

        // Identify affected partitions
        let mut affected_partitions = HashSet::new();
        for missing_version in &missing_versions {
            for (partition_id, versions) in &self.partition_versions {
                if versions.contains(missing_version) {
                    affected_partitions.insert(partition_id.clone());
                }
            }
        }

        (missing_versions, affected_partitions.into_iter().collect())
    }
}

#[test]
fn test_cdf_range_drift_detection() {
    // Test CDF drift detection with missing commit versions
    use uuid::Uuid;

    // Create detector expecting commit versions 1-10
    let expected_versions: Vec<i64> = (1..=10).collect();
    let mut partition_versions = std::collections::HashMap::new();
    partition_versions.insert("partition_1".to_string(), vec![1, 2, 3, 4]);
    partition_versions.insert("partition_2".to_string(), vec![5, 6, 7]);
    partition_versions.insert("partition_3".to_string(), vec![8, 9, 10]);

    let detector =
        CdfDriftDetector::new(expected_versions).with_partition_versions(partition_versions);

    // Create receipts with CDF metadata (missing versions 5-7)
    let receipt1 = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000020").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::Execution,
        dom: "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        cod: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        links: vec![],
        ctx: Context {
            policy_ref: Some("pol:test".to_string()),
            timestamp: "2025-11-08T12:00:00Z".to_string(),
            nonce: None,
            determinism: Some(DeterminismClass::Strict),
            identity_ref: None,
        },
        payload: Payload::Execution(ExecutionPayload {
            trace_id: "tr_cdf_drift_001".to_string(),
            method_ref: MethodRef {
                method_id: "com.acme/cdf_scan".to_string(),
                version: "1.0.0".to_string(),
                method_shape_root:
                    "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                        .to_string(),
            },
            data_shape_hash:
                "sha256:3333333333333333333333333333333333333333333333333333333333333333"
                    .to_string(),
            span_commitments: vec![
                "sha256:4444444444444444444444444444444444444444444444444444444444444444"
                    .to_string(),
            ],
            roots: ExecutionRoots {
                trace_set_root:
                    "sha256:5555555555555555555555555555555555555555555555555555555555555555"
                        .to_string(),
                identity_root:
                    "sha256:6666666666666666666666666666666666666666666666666666666666666666"
                        .to_string(),
                trace_seq_root: None,
            },
            cdf_metadata: Some(vec![
                CdfMetadata {
                    commit_version: 1,
                    change_type: "insert".to_string(),
                    commit_timestamp: "2025-11-08T12:00:00Z".to_string(),
                },
                CdfMetadata {
                    commit_version: 2,
                    change_type: "insert".to_string(),
                    commit_timestamp: "2025-11-08T12:01:00Z".to_string(),
                },
                CdfMetadata {
                    commit_version: 3,
                    change_type: "insert".to_string(),
                    commit_timestamp: "2025-11-08T12:02:00Z".to_string(),
                },
                CdfMetadata {
                    commit_version: 4,
                    change_type: "insert".to_string(),
                    commit_timestamp: "2025-11-08T12:03:00Z".to_string(),
                },
            ]),
            pac: None,
            change_epoch: None,
            minhash_signature: None,
            hll_cardinality: None,
            chunk_manifest_hash: None,
            chunk_manifest_size_bytes: None,
            merkle_root: None,
            prev_execution_rid: None,
            output_digest: None,
            manifest_root: None,
            output_mime_type: None,
            output_size_bytes: None,
            input_locator_refs: None,
            output_locator_ref: None,
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };

    let receipt2 = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000021").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::Execution,
        dom: "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        cod: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        links: vec![],
        ctx: Context {
            policy_ref: Some("pol:test".to_string()),
            timestamp: "2025-11-08T12:00:00Z".to_string(),
            nonce: None,
            determinism: Some(DeterminismClass::Strict),
            identity_ref: None,
        },
        payload: Payload::Execution(ExecutionPayload {
            trace_id: "tr_cdf_drift_002".to_string(),
            method_ref: MethodRef {
                method_id: "com.acme/cdf_scan".to_string(),
                version: "1.0.0".to_string(),
                method_shape_root:
                    "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                        .to_string(),
            },
            data_shape_hash:
                "sha256:3333333333333333333333333333333333333333333333333333333333333333"
                    .to_string(),
            span_commitments: vec![
                "sha256:4444444444444444444444444444444444444444444444444444444444444444"
                    .to_string(),
            ],
            roots: ExecutionRoots {
                trace_set_root:
                    "sha256:5555555555555555555555555555555555555555555555555555555555555555"
                        .to_string(),
                identity_root:
                    "sha256:6666666666666666666666666666666666666666666666666666666666666666"
                        .to_string(),
                trace_seq_root: None,
            },
            cdf_metadata: Some(vec![
                CdfMetadata {
                    commit_version: 8,
                    change_type: "insert".to_string(),
                    commit_timestamp: "2025-11-08T12:07:00Z".to_string(),
                },
                CdfMetadata {
                    commit_version: 9,
                    change_type: "insert".to_string(),
                    commit_timestamp: "2025-11-08T12:08:00Z".to_string(),
                },
                CdfMetadata {
                    commit_version: 10,
                    change_type: "insert".to_string(),
                    commit_timestamp: "2025-11-08T12:09:00Z".to_string(),
                },
            ]),
            pac: None,
            change_epoch: None,
            minhash_signature: None,
            hll_cardinality: None,
            chunk_manifest_hash: None,
            chunk_manifest_size_bytes: None,
            merkle_root: None,
            prev_execution_rid: None,
            output_digest: None,
            manifest_root: None,
            output_mime_type: None,
            output_size_bytes: None,
            input_locator_refs: None,
            output_locator_ref: None,
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };

    // Detect drift
    let (missing_versions, affected_partitions) = detector.detect_cdf_drift(&[receipt1, receipt2]);

    // Verify missing versions 5, 6, 7
    assert_eq!(
        missing_versions.len(),
        3,
        "Should detect 3 missing versions"
    );
    assert!(
        missing_versions.contains(&5),
        "Should detect missing version 5"
    );
    assert!(
        missing_versions.contains(&6),
        "Should detect missing version 6"
    );
    assert!(
        missing_versions.contains(&7),
        "Should detect missing version 6"
    );

    // Verify affected partition
    assert_eq!(
        affected_partitions.len(),
        1,
        "Should identify 1 affected partition"
    );
    assert!(
        affected_partitions.contains(&"partition_2".to_string()),
        "partition_2 should be identified as affected"
    );
}

/// Compare MinHash sketches and detect drift when divergence >5%.
///
/// This function compares two MinHash sketch hashes by computing Jaccard similarity
/// of the underlying chunk sets. If similarity < 0.95 (i.e., divergence >5%),
/// drift is detected.
///
/// # Arguments
///
/// * `sketch1` - First MinHash sketch hash
/// * `sketch2` - Second MinHash sketch hash
/// * `chunk_set1` - Chunk set used to compute sketch1
/// * `chunk_set2` - Chunk set used to compute sketch2
///
/// # Returns
///
/// `true` if drift detected (divergence >5%), `false` otherwise
pub fn detect_minhash_drift(
    chunk_set1: &std::collections::HashSet<String>,
    chunk_set2: &std::collections::HashSet<String>,
) -> bool {
    use std::collections::HashSet;

    // Compute Jaccard similarity
    let intersection: HashSet<_> = chunk_set1.intersection(chunk_set2).cloned().collect();
    let union: HashSet<_> = chunk_set1.union(chunk_set2).cloned().collect();

    let jaccard = if union.is_empty() {
        1.0
    } else {
        intersection.len() as f64 / union.len() as f64
    };

    // Drift detected if Jaccard similarity < 0.95 (divergence >5%)
    jaccard < 0.95
}

#[test]
fn test_minhash_sketch_divergence() {
    use northroot_receipts::canonical::sha256_prefixed;
    use std::collections::HashSet;

    // Helper function to compute chunk ID (same logic as engine, but without dependency)
    fn chunk_id_from_str(content: &str) -> String {
        sha256_prefixed(content.as_bytes())
    }

    // Create two billing runs with 80% overlap (should not trigger drift)
    let run1_tuples = vec![
        "acct1:s3:us-east-1:bucket",
        "acct2:ec2:us-west-2:instance",
        "acct3:rds:eu-west-1:db",
        "acct4:lambda:ap-southeast-1:function",
        "acct5:s3:us-east-1:bucket",
    ];
    let run2_tuples = vec![
        "acct1:s3:us-east-1:bucket",
        "acct2:ec2:us-west-2:instance",
        "acct3:rds:eu-west-1:db",
        "acct4:lambda:ap-southeast-1:function",
        "acct6:ec2:us-east-1:instance", // Different tuple
    ];

    // Convert to chunk sets for comparison
    let chunk_set1: HashSet<String> = run1_tuples.iter().map(|t| chunk_id_from_str(t)).collect();
    let chunk_set2: HashSet<String> = run2_tuples.iter().map(|t| chunk_id_from_str(t)).collect();

    // 4 out of 6 unique tuples overlap = 4/6 = 0.667 < 0.95, so drift should be detected
    // Actually wait, let me recalculate: run1 has 5 tuples, run2 has 5 tuples
    // Intersection: 4 tuples (acct1, acct2, acct3, acct4)
    // Union: 6 unique tuples (acct1, acct2, acct3, acct4, acct5, acct6)
    // Jaccard = 4/6 = 0.667 < 0.95, so drift detected

    let drift_detected = detect_minhash_drift(&chunk_set1, &chunk_set2);
    assert!(
        drift_detected,
        "80% overlap (J=0.667) should trigger drift alert (>5% divergence)"
    );

    // Create two billing runs with 95%+ overlap (should not trigger drift)
    let run3_tuples = vec![
        "acct1:s3:us-east-1:bucket",
        "acct2:ec2:us-west-2:instance",
        "acct3:rds:eu-west-1:db",
        "acct4:lambda:ap-southeast-1:function",
        "acct5:s3:us-east-1:bucket",
    ];
    let run4_tuples = vec![
        "acct1:s3:us-east-1:bucket",
        "acct2:ec2:us-west-2:instance",
        "acct3:rds:eu-west-1:db",
        "acct4:lambda:ap-southeast-1:function",
        "acct5:s3:us-east-1:bucket", // Same as run3
    ];

    let chunk_set3: HashSet<String> = run3_tuples.iter().map(|t| chunk_id_from_str(t)).collect();
    let chunk_set4: HashSet<String> = run4_tuples.iter().map(|t| chunk_id_from_str(t)).collect();

    // 5 out of 5 tuples overlap = 5/5 = 1.0 >= 0.95, so no drift
    let drift_detected2 = detect_minhash_drift(&chunk_set3, &chunk_set4);
    assert!(
        !drift_detected2,
        "100% overlap (J=1.0) should not trigger drift alert"
    );

    // Create two billing runs with 70% overlap (should trigger drift)
    let run5_tuples = vec![
        "acct1:s3:us-east-1:bucket",
        "acct2:ec2:us-west-2:instance",
        "acct3:rds:eu-west-1:db",
        "acct4:lambda:ap-southeast-1:function",
        "acct5:s3:us-east-1:bucket",
    ];
    let run6_tuples = vec![
        "acct1:s3:us-east-1:bucket",
        "acct2:ec2:us-west-2:instance",
        "acct3:rds:eu-west-1:db",
        "acct7:ec2:us-east-1:instance", // Different
        "acct8:s3:us-west-2:bucket",    // Different
    ];

    let chunk_set5: HashSet<String> = run5_tuples.iter().map(|t| chunk_id_from_str(t)).collect();
    let chunk_set6: HashSet<String> = run6_tuples.iter().map(|t| chunk_id_from_str(t)).collect();

    // 3 out of 7 unique tuples overlap = 3/7 = 0.429 < 0.95, so drift detected
    let drift_detected3 = detect_minhash_drift(&chunk_set5, &chunk_set6);
    assert!(
        drift_detected3,
        "70% overlap (J=0.429) should trigger drift alert (>5% divergence)"
    );
}

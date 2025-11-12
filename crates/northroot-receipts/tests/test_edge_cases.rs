//! Edge case tests: optional fields, empty arrays, invalid formats.

use northroot_receipts::*;
use uuid::Uuid;

#[test]
fn test_empty_links() {
    let receipt = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::DataShape,
        dom: "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        cod: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        links: vec![], // Empty links
        ctx: Context {
            policy_ref: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            nonce: None,
            determinism: None,
            identity_ref: None,
        },
        payload: Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                .to_string(),
            sketch_hash: None, // Optional field missing
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };

    let hash = receipt.compute_hash().unwrap();
    let receipt = Receipt { hash, ..receipt };

    receipt.validate().unwrap();
}

#[test]
fn test_all_determinism_classes() {
    let base_receipt = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::DataShape,
        dom: "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        cod: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        links: vec![],
        ctx: Context {
            policy_ref: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            nonce: None,
            determinism: None,
            identity_ref: None,
        },
        payload: Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                .to_string(),
            sketch_hash: None,
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };

    // Test strict
    let mut receipt = base_receipt.clone();
    receipt.ctx.determinism = Some(DeterminismClass::Strict);
    let hash = receipt.compute_hash().unwrap();
    receipt.hash = hash;
    receipt.validate().unwrap();

    // Test bounded
    let mut receipt = base_receipt.clone();
    receipt.ctx.determinism = Some(DeterminismClass::Bounded);
    let hash = receipt.compute_hash().unwrap();
    receipt.hash = hash;
    receipt.validate().unwrap();

    // Test observational
    let mut receipt = base_receipt;
    receipt.ctx.determinism = Some(DeterminismClass::Observational);
    let hash = receipt.compute_hash().unwrap();
    receipt.hash = hash;
    receipt.validate().unwrap();
}

#[test]
fn test_invalid_hash_format() {
    let receipt = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::DataShape,
        dom: "invalid_hash".to_string(), // Invalid format
        cod: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        links: vec![],
        ctx: Context {
            policy_ref: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            nonce: None,
            determinism: None,
            identity_ref: None,
        },
        payload: Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                .to_string(),
            sketch_hash: None,
        }),
        attest: None,
        sig: None,
        hash: "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
    };

    assert!(receipt.validate().is_err());
}

#[test]
fn test_missing_optional_fields() {
    // Method shape without edges
    let receipt = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::MethodShape,
        dom: "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        cod: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        links: vec![],
        ctx: Context {
            policy_ref: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            nonce: None,
            determinism: None,
            identity_ref: None,
        },
        payload: Payload::MethodShape(MethodShapePayload {
            nodes: vec![MethodNodeRef {
                id: "n1".to_string(),
                span_shape_hash:
                    "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                        .to_string(),
            }],
            edges: None, // Optional field missing
            root_multiset:
                "sha256:3333333333333333333333333333333333333333333333333333333333333333"
                    .to_string(),
            dag_hash: None, // Optional field missing
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };

    let hash = receipt.compute_hash().unwrap();
    let receipt = Receipt { hash, ..receipt };

    receipt.validate().unwrap();
}

#[test]
fn test_execution_without_trace_seq_root() {
    let receipt = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::Execution,
        dom: "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        cod: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        links: vec![],
        ctx: Context {
            policy_ref: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            nonce: None,
            determinism: None,
            identity_ref: None,
        },
        payload: Payload::Execution(ExecutionPayload {
            trace_id: "test_trace".to_string(),
            method_ref: MethodRef {
                method_id: "test/method".to_string(),
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
                trace_seq_root: None, // Optional field missing
            },
            cdf_metadata: None,
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

    let hash = receipt.compute_hash().unwrap();
    let receipt = Receipt { hash, ..receipt };

    receipt.validate().unwrap();
}

#[test]
fn test_invalid_timestamp_format() {
    let mut receipt = create_minimal_receipt();
    receipt.ctx.timestamp = "invalid-timestamp".to_string();
    let hash = receipt.compute_hash().unwrap();
    receipt.hash = hash;
    assert!(receipt.validate().is_err());
}

#[test]
fn test_invalid_currency_code() {
    let receipt = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::Spend,
        dom: "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        cod: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        links: vec![],
        ctx: Context {
            policy_ref: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            nonce: None,
            determinism: None,
            identity_ref: None,
        },
        payload: Payload::Spend(SpendPayload {
            meter: ResourceVector {
                vcpu_sec: Some(1.0),
                gpu_sec: None,
                gb_sec: None,
                requests: None,
                energy_kwh: None,
            },
            unit_prices: ResourceVector {
                vcpu_sec: Some(0.1),
                gpu_sec: None,
                gb_sec: None,
                requests: None,
                energy_kwh: None,
            },
            currency: "usd".to_string(),
            pricing_policy_ref: None,
            total_value: 0.1,
            pointers: SpendPointers {
                trace_id: "test".to_string(),
                span_ids: None,
            },
            justification: None,
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };
    let hash = receipt.compute_hash().unwrap();
    let receipt = Receipt { hash, ..receipt };
    assert!(receipt.validate().is_err());
}

#[test]
fn test_invalid_version_format() {
    let mut receipt = create_minimal_receipt();
    receipt.version = "invalid".to_string();
    let hash = receipt.compute_hash().unwrap();
    receipt.hash = hash;
    assert!(receipt.validate().is_err());
}

#[test]
fn test_invalid_policy_ref_format() {
    let mut receipt = create_minimal_receipt();
    receipt.ctx.policy_ref = Some("invalid-policy".to_string());
    let hash = receipt.compute_hash().unwrap();
    receipt.hash = hash;
    assert!(receipt.validate().is_err());
}

#[test]
fn test_invalid_identity_ref_format() {
    let mut receipt = create_minimal_receipt();
    receipt.ctx.identity_ref = Some("invalid-did".to_string());
    let hash = receipt.compute_hash().unwrap();
    receipt.hash = hash;
    assert!(receipt.validate().is_err());
}

#[test]
fn test_valid_policy_ref_legacy_format() {
    let mut receipt = create_minimal_receipt();
    receipt.ctx.policy_ref = Some("pol:standard-v1".to_string());
    let hash = receipt.compute_hash().unwrap();
    receipt.hash = hash;
    receipt.validate().unwrap();
}

#[test]
fn test_valid_policy_ref_strict_format() {
    let mut receipt = create_minimal_receipt();
    receipt.ctx.policy_ref = Some("pol:finops/cost-guard@1.2.0".to_string());
    let hash = receipt.compute_hash().unwrap();
    receipt.hash = hash;
    receipt.validate().unwrap();
}

#[test]
fn test_valid_did_uri() {
    let mut receipt = create_minimal_receipt();
    receipt.ctx.identity_ref =
        Some("did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string());
    let hash = receipt.compute_hash().unwrap();
    receipt.hash = hash;
    receipt.validate().unwrap();
}

#[test]
fn test_valid_did_web() {
    let mut receipt = create_minimal_receipt();
    receipt.ctx.identity_ref = Some("did:web:northroot.dev".to_string());
    let hash = receipt.compute_hash().unwrap();
    receipt.hash = hash;
    receipt.validate().unwrap();
}

fn create_minimal_receipt() -> Receipt {
    Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::DataShape,
        dom: "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        cod: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        links: vec![],
        ctx: Context {
            policy_ref: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            nonce: None,
            determinism: None,
            identity_ref: None,
        },
        payload: Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                .to_string(),
            sketch_hash: None,
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    }
}

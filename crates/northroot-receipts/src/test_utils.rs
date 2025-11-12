//! Test utilities for generating receipt vectors.
//!
//! This module provides shared functions for generating test receipts
//! used by both the generate_vectors and regenerate_vectors test modules.

use crate::{identity_root_from_identities, IdentityRecord, *};
use uuid::Uuid;

/// Generate a data_shape receipt.
pub fn generate_data_shape_receipt() -> Receipt {
    let schema_hash =
        "sha256:68ba79b042ab0cbfcfdfc7cfc0fc1e7b1beeb6d6ddd336a2a88a2f02b64fe414".to_string();

    let receipt = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::DataShape,
        dom: "sha256:385cfdbc00ec32031699460779c15099b2bba3cad0e440fffb08e10df0acb9e1".to_string(),
        cod: schema_hash.clone(),
        links: vec![],
        ctx: Context {
            policy_ref: Some("pol:standard-v1".to_string()),
            timestamp: "2025-11-06T12:00:00.000Z".to_string(),
            nonce: None,
            determinism: Some(DeterminismClass::Strict),
            identity_ref: Some("did:key:zNorthroot".to_string()),
        },
        payload: Payload::DataShape(DataShapePayload {
            schema_hash,
            sketch_hash: None,
        }),
        attest: None,
        sig: None,
        hash: String::new(), // Will be computed
    };

    let hash = receipt.compute_hash().unwrap();
    Receipt { hash, ..receipt }
}

/// Generate a method_shape receipt.
pub fn generate_method_shape_receipt(prev_cod: &str) -> Receipt {
    let receipt = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::MethodShape,
        dom: prev_cod.to_string(),
        cod: "sha256:e7880638dcc10d2f2a4a817d8aeb1ea618a2b3f7c6b6d730f4f8c06f586ea81b".to_string(),
        links: vec![Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()],
        ctx: Context {
            policy_ref: Some("pol:standard-v1".to_string()),
            timestamp: "2025-11-06T12:00:00.000Z".to_string(),
            nonce: None,
            determinism: Some(DeterminismClass::Strict),
            identity_ref: Some("did:key:zNorthroot".to_string()),
        },
        payload: Payload::MethodShape(MethodShapePayload {
            nodes: vec![
                MethodNodeRef {
                    id: "n1".to_string(),
                    span_shape_hash:
                        "sha256:b5c142d31729effb5b42559f64c42f0cdd2693eca04afb9eccb65a8217902634"
                            .to_string(),
                },
                MethodNodeRef {
                    id: "n2".to_string(),
                    span_shape_hash:
                        "sha256:1a1802af391d05d8d39f4c6d727523f72ca99bc53cf2bc91ac876c01a82e4156"
                            .to_string(),
                },
            ],
            edges: Some(vec![Edge {
                from: "n1".to_string(),
                to: "n2".to_string(),
            }]),
            root_multiset:
                "sha256:a01494edccade55e39cd74ca38709b344295d908e5c83981bb7e23dc7b311c15"
                    .to_string(),
            dag_hash: Some(
                "sha256:d01db50cc0ceb288c8c756289f0d4f8ee0f753dd8cf187f07e01164724060d4e"
                    .to_string(),
            ),
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };

    let hash = receipt.compute_hash().unwrap();
    Receipt { hash, ..receipt }
}

/// Generate a reasoning_shape receipt.
pub fn generate_reasoning_shape_receipt(prev_cod: &str) -> Receipt {
    let receipt = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::ReasoningShape,
        dom: prev_cod.to_string(),
        cod: "sha256:fb04389691fdb79ff2df1b9868e10764001991e1ce5342504b80e482e088b6b5".to_string(),
        links: vec![Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap()],
        ctx: Context {
            policy_ref: Some("pol:standard-v1".to_string()),
            timestamp: "2025-11-06T12:00:00.000Z".to_string(),
            nonce: None,
            determinism: Some(DeterminismClass::Strict),
            identity_ref: Some("did:key:zNorthroot".to_string()),
        },
        payload: Payload::ReasoningShape(ReasoningShapePayload {
            intent_hash: "sha256:40c74ab648736d7f2ec429ac6d72ea87167688af1f803ad02101d6b85f2a0ab4"
                .to_string(),
            dag_hash: "sha256:fb04389691fdb79ff2df1b9868e10764001991e1ce5342504b80e482e088b6b5"
                .to_string(),
            node_refs: vec![
                ReasoningNodeRef {
                    node_id: "p1".to_string(),
                    operator_ref: "ns.io/read_csv@1".to_string(),
                    pox_ref: None,
                },
                ReasoningNodeRef {
                    node_id: "p2".to_string(),
                    operator_ref: "ns.fin/normalize_amount@1".to_string(),
                    pox_ref: None,
                },
            ],
            policy_ref: Some("pol:standard-v1".to_string()),
            quality: Some(ReasoningQuality {
                success_score: None,
                eval_method: None,
                review_hash: None,
                confidence: None,
            }),
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };

    let hash = receipt.compute_hash().unwrap();
    Receipt { hash, ..receipt }
}

/// Generate an execution receipt.
pub fn generate_execution_receipt(prev_cod: &str) -> Receipt {
    // Compute identity_root from identity records
    // Using the identity from ctx.identity_ref as the primary executor
    let identities = vec![IdentityRecord {
        did: "did:key:zNorthroot".to_string(),
        kid: "did:key:zNorthroot#key-1".to_string(),
        role: Some("executor".to_string()),
        tenant: None,
    }];
    let identity_root = identity_root_from_identities(identities.into_iter());

    let receipt = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000004").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::Execution,
        dom: prev_cod.to_string(),
        cod: "sha256:df82f595ae27ea901b2ad1f559025a405ccd285cae382a78da09f01a8a89058d".to_string(),
        links: vec![Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap()],
        ctx: Context {
            policy_ref: Some("pol:standard-v1".to_string()),
            timestamp: "2025-11-06T12:00:00.000Z".to_string(),
            nonce: None,
            determinism: Some(DeterminismClass::Strict),
            identity_ref: Some("did:key:zNorthroot".to_string()),
        },
        payload: Payload::Execution(ExecutionPayload {
            trace_id: "tr_demo_001".to_string(),
            method_ref: MethodRef {
                method_id: "com.acme/normalize-ledger".to_string(),
                version: "1.0.0".to_string(),
                method_shape_root:
                    "sha256:a01494edccade55e39cd74ca38709b344295d908e5c83981bb7e23dc7b311c15"
                        .to_string(),
            },
            data_shape_hash:
                "sha256:68ba79b042ab0cbfcfdfc7cfc0fc1e7b1beeb6d6ddd336a2a88a2f02b64fe414"
                    .to_string(),
            span_commitments: vec![
                "sha256:9897d53ba0aa43e2e9f2d935591ce280da9fa8da27823ac2659fc4264e94fba7"
                    .to_string(),
                "sha256:e2f03f888ccf3e36d4f17e50d9a4b0c01957bac865eaf96e6419b97c3f3d63a0"
                    .to_string(),
            ],
            roots: ExecutionRoots {
                trace_set_root:
                    "sha256:df82f595ae27ea901b2ad1f559025a405ccd285cae382a78da09f01a8a89058d"
                        .to_string(),
                identity_root,
                trace_seq_root: Some(
                    "sha256:df82f595ae27ea901b2ad1f559025a405ccd285cae382a78da09f01a8a89058d"
                        .to_string(),
                ),
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
    Receipt { hash, ..receipt }
}

/// Generate a spend receipt.
pub fn generate_spend_receipt(prev_cod: &str) -> Receipt {
    let receipt = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000005").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::Spend,
        dom: prev_cod.to_string(),
        cod: "sha256:dd06220b9b4521452dea8c603219d0b549aa07357835d26519cc03bdb2dbefd7".to_string(),
        links: vec![Uuid::parse_str("00000000-0000-0000-0000-000000000004").unwrap()],
        ctx: Context {
            policy_ref: Some("pol:standard-v1".to_string()),
            timestamp: "2025-11-06T12:00:00.000Z".to_string(),
            nonce: None,
            determinism: Some(DeterminismClass::Strict),
            identity_ref: Some("did:key:zNorthroot".to_string()),
        },
        payload: Payload::Spend(SpendPayload {
            meter: ResourceVector {
                vcpu_sec: Some(0.1),
                gpu_sec: None,
                gb_sec: Some(0.02),
                requests: Some(2.0),
                energy_kwh: None,
            },
            unit_prices: ResourceVector {
                vcpu_sec: Some(0.05),
                gpu_sec: None,
                gb_sec: Some(0.01),
                requests: Some(0.001),
                energy_kwh: None,
            },
            currency: "USD".to_string(),
            pricing_policy_ref: Some("price:std-v1".to_string()),
            total_value: 0.0072, // 0.1*0.05 + 0.02*0.01 + 2.0*0.001 = 0.005 + 0.0002 + 0.002 = 0.0072
            pointers: SpendPointers {
                trace_id: "tr_demo_001".to_string(),
                span_ids: Some(vec!["sp-0001".to_string(), "sp-0002".to_string()]),
            },
            justification: Some(ReuseJustification {
                overlap_j: Some(0.9),
                alpha: Some(0.9),
                c_id: Some(0.0005),
                c_comp: Some(0.01),
                decision: Some("reuse".to_string()),
                layer: Some("data".to_string()),
                minhash_sketch: None,
            }),
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };

    let hash = receipt.compute_hash().unwrap();
    Receipt { hash, ..receipt }
}

/// Generate a settlement receipt.
pub fn generate_settlement_receipt(prev_cod: &str) -> Receipt {
    use std::collections::BTreeMap;

    let mut net_positions = BTreeMap::new();
    net_positions.insert("did:key:partyA".to_string(), 0.72);
    net_positions.insert("did:key:partyB".to_string(), -0.72);

    let receipt = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000006").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::Settlement,
        dom: prev_cod.to_string(),
        cod: "sha256:346ae4ca888b80b0220ccc02a5fc3798e5525b30b77083f784e003d3106ae107".to_string(),
        links: vec![Uuid::parse_str("00000000-0000-0000-0000-000000000005").unwrap()],
        ctx: Context {
            policy_ref: Some("pol:standard-v1".to_string()),
            timestamp: "2025-11-06T12:00:00.000Z".to_string(),
            nonce: None,
            determinism: Some(DeterminismClass::Strict),
            identity_ref: Some("did:key:zNorthroot".to_string()),
        },
        payload: Payload::Settlement(SettlementPayload {
            wur_refs: vec![
                "00000000-0000-0000-0000-000000000004".to_string(),
                "00000000-0000-0000-0000-000000000005".to_string(),
            ],
            net_positions,
            rules_ref: "clear:net-v1".to_string(),
            cash_instr: {
                // Convert JSON to CBOR Value for test
                let json_val = serde_json::json!({
                "method": "ach",
                "routing": "021000021",
                "account": "****1234"
                });
                let mut cbor_bytes = Vec::new();
                if ciborium::ser::into_writer(&json_val, &mut cbor_bytes).is_ok() {
                    ciborium::de::from_reader(cbor_bytes.as_slice()).ok()
                } else {
                    None
                }
            },
        }),
        attest: None,
        sig: None,
        hash: String::new(),
    };

    let hash = receipt.compute_hash().unwrap();
    Receipt { hash, ..receipt }
}

/// Generate a sequential chain of receipts with matching dom/cod.
pub fn generate_sequential_chain() -> Vec<Receipt> {
    let data_shape = generate_data_shape_receipt();
    let method_shape = generate_method_shape_receipt(&data_shape.cod);
    let reasoning_shape = generate_reasoning_shape_receipt(&method_shape.cod);
    let execution = generate_execution_receipt(&reasoning_shape.cod);
    let spend = generate_spend_receipt(&execution.cod);
    let settlement = generate_settlement_receipt(&spend.cod);

    vec![
        data_shape,
        method_shape,
        reasoning_shape,
        execution,
        spend,
        settlement,
    ]
}

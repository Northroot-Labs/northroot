//! Tests for SQLite storage backend.

use northroot_receipts::{Context, ExecutionPayload, MethodRef, Payload, Receipt, ReceiptKind};
use northroot_storage::{ReceiptQuery, ReceiptStore, SqliteStore};
use uuid::Uuid;

fn create_test_receipt(rid: Uuid) -> Receipt {
    let method_ref = MethodRef {
        method_id: "com.test/example".to_string(),
        version: "1.0.0".to_string(),
        method_shape_root:
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
    };

    let payload = Payload::Execution(ExecutionPayload {
        trace_id: "test-trace-1".to_string(),
        method_ref: method_ref.clone(),
        data_shape_hash: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
            .to_string(),
        span_commitments: vec![
            "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        ],
        roots: northroot_receipts::ExecutionRoots {
            trace_set_root:
                "sha256:4444444444444444444444444444444444444444444444444444444444444444"
                    .to_string(),
            trace_seq_root: Some(
                "sha256:6666666666666666666666666666666666666666666666666666666666666666"
                    .to_string(),
            ),
            identity_root:
                "sha256:5555555555555555555555555555555555555555555555555555555555555555"
                    .to_string(),
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
    });

    let ctx = Context {
        policy_ref: Some("pol:test@1".to_string()),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        nonce: None,
        determinism: None,
        identity_ref: None,
    };

    let mut receipt = Receipt {
        rid,
        version: "0.3.0".to_string(),
        kind: ReceiptKind::Execution,
        dom: "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        cod: "sha256:4444444444444444444444444444444444444444444444444444444444444444".to_string(),
        links: Vec::new(),
        ctx,
        payload,
        attest: None,
        sig: None,
        hash: String::new(),
    };

    receipt.hash = receipt.compute_hash().unwrap();
    receipt
}

#[test]
fn test_store_and_retrieve_receipt() {
    let store = SqliteStore::in_memory().unwrap();
    let rid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let receipt = create_test_receipt(rid);

    // Store receipt
    store.store_receipt(&receipt).unwrap();

    // Retrieve receipt
    let retrieved = store.get_receipt(&rid).unwrap().unwrap();
    assert_eq!(retrieved.rid, rid);
    assert_eq!(retrieved.hash, receipt.hash);
}

#[test]
fn test_query_receipts_by_policy() {
    let store = SqliteStore::in_memory().unwrap();

    let rid1 = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let rid2 = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

    let receipt1 = create_test_receipt(rid1);
    let mut receipt2 = create_test_receipt(rid2);
    receipt2.ctx.policy_ref = Some("pol:other@1".to_string());
    receipt2.hash = receipt2.compute_hash().unwrap();

    store.store_receipt(&receipt1).unwrap();
    store.store_receipt(&receipt2).unwrap();

    let query = ReceiptQuery {
        policy_ref: Some("pol:test@1".to_string()),
        ..Default::default()
    };
    let results = store.query_receipts(query).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].rid, rid1);
}

#[test]
fn test_store_and_retrieve_manifest() {
    let store = SqliteStore::in_memory().unwrap();

    let hash = [0u8; 32];
    let data = b"test manifest data";
    let meta = northroot_storage::ManifestMeta {
        pac: [1u8; 32],
        change_epoch_id: Some("snap-123".to_string()),
        encoding: "zstd".to_string(),
        size_uncompressed: data.len(),
        expires_at: None,
    };

    store.put_manifest(&hash, data, &meta).unwrap();

    let retrieved = store.get_manifest(&hash).unwrap().unwrap();
    assert_eq!(retrieved, data);
}

#[test]
fn test_gc_manifests() {
    let store = SqliteStore::in_memory().unwrap();

    let hash1 = [0u8; 32];
    let hash2 = [1u8; 32];
    let data = b"test data";

    let meta1 = northroot_storage::ManifestMeta {
        pac: [1u8; 32],
        change_epoch_id: None,
        encoding: "zstd".to_string(),
        size_uncompressed: data.len(),
        expires_at: Some(1000), // Expired
    };

    let meta2 = northroot_storage::ManifestMeta {
        pac: [2u8; 32],
        change_epoch_id: None,
        encoding: "zstd".to_string(),
        size_uncompressed: data.len(),
        expires_at: Some(9999999999), // Not expired
    };

    store.put_manifest(&hash1, data, &meta1).unwrap();
    store.put_manifest(&hash2, data, &meta2).unwrap();

    let count = store.gc_manifests(2000).unwrap();
    assert_eq!(count, 1);

    // First manifest should be gone
    assert!(store.get_manifest(&hash1).unwrap().is_none());
    // Second manifest should still exist
    assert!(store.get_manifest(&hash2).unwrap().is_some());
}

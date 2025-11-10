//! Tests for signature verification.

use northroot_engine::signature::*;
use northroot_receipts::{Context, DataShapePayload, Payload, Receipt, ReceiptKind, Signature};
use uuid::Uuid;

fn create_test_receipt_with_sig(hash: String, sig: Option<Signature>) -> Receipt {
    let ctx = Context {
        policy_ref: None,
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        nonce: None,
        determinism: None,
        identity_ref: None,
    };

    let payload = Payload::DataShape(DataShapePayload {
        schema_hash: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
            .to_string(),
        sketch_hash: None,
    });

    Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::DataShape,
        dom: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        cod: "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        links: Vec::new(),
        ctx,
        payload,
        attest: None,
        sig,
        hash,
    }
}

#[test]
fn test_verify_signature_missing() {
    let receipt = create_test_receipt_with_sig(
        "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        None,
    );

    let result = verify_signature(&receipt);
    assert!(result.is_err());
    match result.unwrap_err() {
        SignatureError::MissingSignature => {}
        _ => panic!("Expected MissingSignature error"),
    }
}

#[test]
fn test_verify_signature_invalid_hash_format() {
    let sig = Signature {
        alg: "ed25519".to_string(),
        kid: "did:key:zTest".to_string(),
        sig: "test".to_string(),
    };

    let receipt = create_test_receipt_with_sig("invalid_hash".to_string(), Some(sig));

    let result = verify_signature(&receipt);
    assert!(result.is_err());
    match result.unwrap_err() {
        SignatureError::InvalidFormat(_) => {}
        _ => panic!("Expected InvalidFormat error"),
    }
}

#[test]
fn test_verify_signature_unsupported_algorithm() {
    let sig = Signature {
        alg: "rsa".to_string(),
        kid: "did:key:zTest".to_string(),
        sig: "test".to_string(),
    };

    let receipt = create_test_receipt_with_sig(
        "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        Some(sig),
    );

    let result = verify_signature(&receipt);
    assert!(result.is_err());
    match result.unwrap_err() {
        SignatureError::UnsupportedAlgorithm { alg } => {
            assert_eq!(alg, "rsa");
        }
        _ => panic!("Expected UnsupportedAlgorithm error"),
    }
}

#[test]
fn test_resolve_did_key_invalid_format() {
    let result = resolve_did_key("not-a-did");
    assert!(result.is_err());
    match result.unwrap_err() {
        SignatureError::DidResolutionFailed { .. } => {}
        _ => panic!("Expected DidResolutionFailed error"),
    }
}

#[test]
fn test_resolve_did_key_missing_z_prefix() {
    let result = resolve_did_key("did:key:invalid");
    assert!(result.is_err());
}

#[test]
fn test_resolve_did_key_empty() {
    let result = resolve_did_key("did:key:");
    assert!(result.is_err());
}

#[test]
fn test_verify_all_signatures() {
    // Currently just calls verify_signature, so test is similar
    let receipt = create_test_receipt_with_sig(
        "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        None,
    );

    let result = verify_all_signatures(&receipt);
    assert!(result.is_err());
    match result.unwrap_err() {
        SignatureError::MissingSignature => {}
        _ => panic!("Expected MissingSignature error"),
    }
}

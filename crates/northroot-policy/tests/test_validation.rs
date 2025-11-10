//! Tests for policy validation.

use northroot_policy::validation::*;
use northroot_receipts::{
    Context, DataShapePayload, DeterminismClass, Payload, Receipt, ReceiptKind,
};
use uuid::Uuid;

#[test]
fn test_validate_policy_ref_format_strict() {
    assert!(validate_policy_ref_format("pol:finops/cost-guard@1.2.0").is_ok());
    assert!(validate_policy_ref_format("pol:namespace/name@2.0.1").is_ok());
    assert!(validate_policy_ref_format("pol:my-namespace/my-policy@0.1.0").is_ok());
}

#[test]
fn test_validate_policy_ref_format_legacy() {
    assert!(validate_policy_ref_format("pol:standard-v1").is_ok());
    assert!(validate_policy_ref_format("pol:my-policy-v2").is_ok());
    assert!(validate_policy_ref_format("pol:test_policy-v3").is_ok());
}

#[test]
fn test_validate_policy_ref_format_invalid() {
    assert!(validate_policy_ref_format("invalid").is_err());
    assert!(validate_policy_ref_format("pol:").is_err());
    assert!(validate_policy_ref_format("pol:namespace/name").is_err()); // Missing @version
    assert!(validate_policy_ref_format("pol:/name@1.0.0").is_err()); // Empty namespace
    assert!(validate_policy_ref_format("pol:namespace/@1.0.0").is_err()); // Empty name
}

#[test]
fn test_validate_determinism_strict_required() {
    let ctx_strict = Context {
        policy_ref: None,
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        nonce: None,
        determinism: Some(DeterminismClass::Strict),
        identity_ref: None,
    };

    let ctx_bounded = Context {
        determinism: Some(DeterminismClass::Bounded),
        ..ctx_strict.clone()
    };

    let ctx_observational = Context {
        determinism: Some(DeterminismClass::Observational),
        ..ctx_strict.clone()
    };

    let ctx_none = Context {
        determinism: None,
        ..ctx_strict.clone()
    };

    assert!(validate_determinism(&ctx_strict, DeterminismClass::Strict).is_ok());
    assert!(validate_determinism(&ctx_bounded, DeterminismClass::Strict).is_err());
    assert!(validate_determinism(&ctx_observational, DeterminismClass::Strict).is_err());
    assert!(validate_determinism(&ctx_none, DeterminismClass::Strict).is_err());
}

#[test]
fn test_validate_determinism_bounded_required() {
    let ctx_strict = Context {
        policy_ref: None,
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        nonce: None,
        determinism: Some(DeterminismClass::Strict),
        identity_ref: None,
    };

    let ctx_bounded = Context {
        determinism: Some(DeterminismClass::Bounded),
        ..ctx_strict.clone()
    };

    let ctx_observational = Context {
        determinism: Some(DeterminismClass::Observational),
        ..ctx_strict.clone()
    };

    let ctx_none = Context {
        determinism: None,
        ..ctx_strict.clone()
    };

    // Bounded accepts strict or bounded
    assert!(validate_determinism(&ctx_strict, DeterminismClass::Bounded).is_ok());
    assert!(validate_determinism(&ctx_bounded, DeterminismClass::Bounded).is_ok());
    assert!(validate_determinism(&ctx_observational, DeterminismClass::Bounded).is_err());
    assert!(validate_determinism(&ctx_none, DeterminismClass::Bounded).is_err());
}

#[test]
fn test_validate_determinism_observational_required() {
    let ctx_strict = Context {
        policy_ref: None,
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        nonce: None,
        determinism: Some(DeterminismClass::Strict),
        identity_ref: None,
    };

    let ctx_bounded = Context {
        determinism: Some(DeterminismClass::Bounded),
        ..ctx_strict.clone()
    };

    let ctx_observational = Context {
        determinism: Some(DeterminismClass::Observational),
        ..ctx_strict.clone()
    };

    let ctx_none = Context {
        determinism: None,
        ..ctx_strict.clone()
    };

    // Observational accepts any class
    assert!(validate_determinism(&ctx_strict, DeterminismClass::Observational).is_ok());
    assert!(validate_determinism(&ctx_bounded, DeterminismClass::Observational).is_ok());
    assert!(validate_determinism(&ctx_observational, DeterminismClass::Observational).is_ok());
    assert!(validate_determinism(&ctx_none, DeterminismClass::Observational).is_ok());
}

#[test]
fn test_load_policy_stub() {
    // Currently returns PolicyNotFound since registry is not implemented
    let result = load_policy("pol:test/policy@1.0.0");
    assert!(result.is_err());
    match result.unwrap_err() {
        PolicyError::PolicyNotFound { policy_ref } => {
            assert_eq!(policy_ref, "pol:test/policy@1.0.0");
        }
        _ => panic!("Expected PolicyNotFound error"),
    }
}

#[test]
fn test_validate_policy_no_policy_ref() {
    let ctx = Context {
        policy_ref: None,
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        nonce: None,
        determinism: None,
        identity_ref: None,
    };

    let receipt = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::DataShape,
        dom: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        cod: "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        links: Vec::new(),
        ctx,
        payload: Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                .to_string(),
            sketch_hash: None,
        }),
        attest: None,
        sig: None,
        hash: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
    };

    // No policy ref in receipt and none provided
    let result = validate_policy(&receipt, None);
    assert!(result.is_err());
    match result.unwrap_err() {
        PolicyError::InvalidPolicyRef { .. } => {}
        _ => panic!("Expected InvalidPolicyRef error"),
    }
}

#[test]
fn test_validate_policy_with_policy_ref() {
    let ctx = Context {
        policy_ref: Some("pol:test/policy@1.0.0".to_string()),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        nonce: None,
        determinism: None,
        identity_ref: None,
    };

    let receipt = Receipt {
        rid: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        version: "0.3.0".to_string(),
        kind: ReceiptKind::DataShape,
        dom: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        cod: "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        links: Vec::new(),
        ctx,
        payload: Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                .to_string(),
            sketch_hash: None,
        }),
        attest: None,
        sig: None,
        hash: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
    };

    // Policy format is valid but policy not found (stub)
    let result = validate_policy(&receipt, None);
    assert!(result.is_err());
    match result.unwrap_err() {
        PolicyError::PolicyNotFound { .. } => {}
        _ => panic!("Expected PolicyNotFound error"),
    }
}

#[test]
fn test_validate_tool_constraints_stub() {
    let policy = serde_json::json!({});
    // Currently stub returns Ok
    assert!(validate_tool_constraints("test-tool", &policy).is_ok());
}

#[test]
fn test_validate_region_constraints_stub() {
    let policy = serde_json::json!({});
    // Currently stub returns Ok
    assert!(validate_region_constraints("us-east-1", &policy).is_ok());
}

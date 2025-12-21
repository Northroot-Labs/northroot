use northroot_canonical::{
    Canonicalizer, Digest, DigestAlg, PrincipalId, ProfileId, Quantity, Timestamp, ToolName,
};
use northroot_core::{
    events::{
        AuthorizationEvent, AuthorizationKind, Decision, ExecutionEvent, GrantBounds, Outcome,
    },
    shared::{IntentAnchors, Meter},
    verification::Verifier,
    compute_event_id, VerificationVerdict,
};

fn make_canonicalizer() -> Canonicalizer {
    Canonicalizer::new(ProfileId::parse("northroot-canonical-v1").unwrap())
}

fn make_digest() -> Digest {
    Digest::new(DigestAlg::Sha256, "dGVzdF9kaWdlc3RfZm9yX3Rlc3RpbmdfcHVycG9zZXM").unwrap()
}

fn compute_event_id_for_test<T: serde::Serialize>(value: &T) -> Digest {
    let canonicalizer = make_canonicalizer();
    compute_event_id(value, &canonicalizer).unwrap()
}

fn make_timestamp() -> Timestamp {
    Timestamp::parse("2024-01-01T00:00:00Z").unwrap()
}

fn make_principal() -> PrincipalId {
    PrincipalId::parse("service:test").unwrap()
}

fn make_profile() -> ProfileId {
    ProfileId::parse("northroot-canonical-v1").unwrap()
}

fn make_intent_anchors() -> IntentAnchors {
    IntentAnchors {
        intent_digest: make_digest(),
        intent_ref: None,
        user_intent_digest: None,
    }
}

#[test]
fn test_same_unit_comparison_within_bounds() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);

    // Create a simple authorization with Int cap
    let mut auth_event = AuthorizationEvent {
        event_id: make_digest(), // Placeholder, will be computed
        event_type: "authorization".to_string(),
        event_version: "1".to_string(),
        prev_event_id: None,
        occurred_at: make_timestamp(),
        principal_id: make_principal(),
        canonical_profile_id: make_profile(),
        intents: make_intent_anchors(),
        policy_id: "test-policy".to_string(),
        policy_digest: make_digest(),
        decision: Decision::Allow,
        decision_code: "ALLOW".to_string(),
        checks: None,
        hygiene: None,
        authorization: AuthorizationKind::Grant {
            bounds: GrantBounds {
                expires_at: None,
                allowed_tools: vec!["test.tool".to_string()],
                meter_caps: vec![Meter {
                    unit: "tokens.input".to_string(),
                    amount: Quantity::int("1000").unwrap(),
                }],
                rate_limits: None,
                concurrency_limit: None,
                output_mode: None,
                resources: None,
            },
        },
    };
    auth_event.event_id = compute_event_id_for_test(&auth_event);

    // Create execution with usage within bounds
    let mut exec_event = ExecutionEvent {
        event_id: make_digest(), // Placeholder
        event_type: "execution".to_string(),
        event_version: "1".to_string(),
        prev_event_id: None,
        occurred_at: make_timestamp(),
        principal_id: make_principal(),
        canonical_profile_id: make_profile(),
        intents: make_intent_anchors(),
        auth_event_id: auth_event.event_id.clone(),
        tool_name: ToolName::parse("test.tool").unwrap(),
        started_at: None,
        ended_at: None,
        meter_used: vec![Meter {
            unit: "tokens.input".to_string(),
            amount: Quantity::int("500").unwrap(),
        }],
        outcome: Outcome::Success,
        error_code: None,
        output_digest: None,
        output_ref: None,
        resources_touched: None,
        model_id: None,
        provider: None,
        pricing_snapshot_digest: None,
    };
    exec_event.event_id = compute_event_id_for_test(&exec_event);

    let (_, verdict) = verifier
        .verify_execution(&exec_event, &auth_event)
        .unwrap();
    assert_eq!(verdict, VerificationVerdict::Ok);
}

#[test]
fn test_same_unit_comparison_exceeds_bounds() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);

    let mut auth_event = AuthorizationEvent {
        event_id: make_digest(),
        event_type: "authorization".to_string(),
        event_version: "1".to_string(),
        prev_event_id: None,
        occurred_at: make_timestamp(),
        principal_id: make_principal(),
        canonical_profile_id: make_profile(),
        intents: make_intent_anchors(),
        policy_id: "test-policy".to_string(),
        policy_digest: make_digest(),
        decision: Decision::Allow,
        decision_code: "ALLOW".to_string(),
        checks: None,
        hygiene: None,
        authorization: AuthorizationKind::Grant {
            bounds: GrantBounds {
                expires_at: None,
                allowed_tools: vec!["test.tool".to_string()],
                meter_caps: vec![Meter {
                    unit: "tokens.input".to_string(),
                    amount: Quantity::int("1000").unwrap(),
                }],
                rate_limits: None,
                concurrency_limit: None,
                output_mode: None,
                resources: None,
            },
        },
    };
    auth_event.event_id = compute_event_id_for_test(&auth_event);

    // Execution with usage exceeding cap
    let mut exec_event = ExecutionEvent {
        event_id: make_digest(),
        event_type: "execution".to_string(),
        event_version: "1".to_string(),
        prev_event_id: None,
        occurred_at: make_timestamp(),
        principal_id: make_principal(),
        canonical_profile_id: make_profile(),
        intents: make_intent_anchors(),
        auth_event_id: auth_event.event_id.clone(),
        tool_name: ToolName::parse("test.tool").unwrap(),
        started_at: None,
        ended_at: None,
        meter_used: vec![Meter {
            unit: "tokens.input".to_string(),
            amount: Quantity::int("1500").unwrap(),
        }],
        outcome: Outcome::Success,
        error_code: None,
        output_digest: None,
        output_ref: None,
        resources_touched: None,
        model_id: None,
        provider: None,
        pricing_snapshot_digest: None,
    };
    exec_event.event_id = compute_event_id_for_test(&exec_event);

    let (_, verdict) = verifier
        .verify_execution(&exec_event, &auth_event)
        .unwrap();
    assert_eq!(verdict, VerificationVerdict::Violation);
}

#[test]
fn test_same_unit_dec_comparison() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);

    let mut auth_event = AuthorizationEvent {
        event_id: make_digest(),
        event_type: "authorization".to_string(),
        event_version: "1".to_string(),
        prev_event_id: None,
        occurred_at: make_timestamp(),
        principal_id: make_principal(),
        canonical_profile_id: make_profile(),
        intents: make_intent_anchors(),
        policy_id: "test-policy".to_string(),
        policy_digest: make_digest(),
        decision: Decision::Allow,
        decision_code: "ALLOW".to_string(),
        checks: None,
        hygiene: None,
        authorization: AuthorizationKind::Grant {
            bounds: GrantBounds {
                expires_at: None,
                allowed_tools: vec!["test.tool".to_string()],
                meter_caps: vec![Meter {
                    unit: "usd".to_string(),
                    amount: Quantity::dec("10000", 2).unwrap(), // $100.00
                }],
                rate_limits: None,
                concurrency_limit: None,
                output_mode: None,
                resources: None,
            },
        },
    };
    auth_event.event_id = compute_event_id_for_test(&auth_event);

    // Execution with USD usage within bounds
    let mut exec_event = ExecutionEvent {
        event_id: make_digest(),
        event_type: "execution".to_string(),
        event_version: "1".to_string(),
        prev_event_id: None,
        occurred_at: make_timestamp(),
        principal_id: make_principal(),
        canonical_profile_id: make_profile(),
        intents: make_intent_anchors(),
        auth_event_id: auth_event.event_id.clone(),
        tool_name: ToolName::parse("test.tool").unwrap(),
        started_at: None,
        ended_at: None,
        meter_used: vec![Meter {
            unit: "usd".to_string(),
            amount: Quantity::dec("5000", 2).unwrap(), // $50.00
        }],
        outcome: Outcome::Success,
        error_code: None,
        output_digest: None,
        output_ref: None,
        resources_touched: None,
        model_id: None,
        provider: None,
        pricing_snapshot_digest: None,
    };
    exec_event.event_id = compute_event_id_for_test(&exec_event);

    let (_, verdict) = verifier
        .verify_execution(&exec_event, &auth_event)
        .unwrap();
    assert_eq!(verdict, VerificationVerdict::Ok);
}

#[test]
fn test_mixed_type_quantities_invalid() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);

    let mut auth_event = AuthorizationEvent {
        event_id: make_digest(),
        event_type: "authorization".to_string(),
        event_version: "1".to_string(),
        prev_event_id: None,
        occurred_at: make_timestamp(),
        principal_id: make_principal(),
        canonical_profile_id: make_profile(),
        intents: make_intent_anchors(),
        policy_id: "test-policy".to_string(),
        policy_digest: make_digest(),
        decision: Decision::Allow,
        decision_code: "ALLOW".to_string(),
        checks: None,
        hygiene: None,
        authorization: AuthorizationKind::Grant {
            bounds: GrantBounds {
                expires_at: None,
                allowed_tools: vec!["test.tool".to_string()],
                meter_caps: vec![Meter {
                    unit: "tokens.input".to_string(),
                    amount: Quantity::int("1000").unwrap(),
                }],
                rate_limits: None,
                concurrency_limit: None,
                output_mode: None,
                resources: None,
            },
        },
    };
    auth_event.event_id = compute_event_id_for_test(&auth_event);

    // Execution with Dec quantity but cap is Int (mixed types)
    let mut exec_event = ExecutionEvent {
        event_id: make_digest(),
        event_type: "execution".to_string(),
        event_version: "1".to_string(),
        prev_event_id: None,
        occurred_at: make_timestamp(),
        principal_id: make_principal(),
        canonical_profile_id: make_profile(),
        intents: make_intent_anchors(),
        auth_event_id: auth_event.event_id.clone(),
        tool_name: ToolName::parse("test.tool").unwrap(),
        started_at: None,
        ended_at: None,
        meter_used: vec![Meter {
            unit: "tokens.input".to_string(),
            amount: Quantity::dec("500", 2).unwrap(), // Dec type with scale 2
        }],
        outcome: Outcome::Success,
        error_code: None,
        output_digest: None,
        output_ref: None,
        resources_touched: None,
        model_id: None,
        provider: None,
        pricing_snapshot_digest: None,
    };
    exec_event.event_id = compute_event_id_for_test(&exec_event);

    let (_, verdict) = verifier
        .verify_execution(&exec_event, &auth_event)
        .unwrap();
    // Mixed types should result in Invalid (no implicit coercion)
    assert_eq!(verdict, VerificationVerdict::Invalid);
}

#[test]
fn test_usd_cap_without_conversion_context() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);

    let mut auth_event = AuthorizationEvent {
        event_id: make_digest(),
        event_type: "authorization".to_string(),
        event_version: "1".to_string(),
        prev_event_id: None,
        occurred_at: make_timestamp(),
        principal_id: make_principal(),
        canonical_profile_id: make_profile(),
        intents: make_intent_anchors(),
        policy_id: "test-policy".to_string(),
        policy_digest: make_digest(),
        decision: Decision::Allow,
        decision_code: "ALLOW".to_string(),
        checks: None,
        hygiene: None,
        authorization: AuthorizationKind::Grant {
            bounds: GrantBounds {
                expires_at: None,
                allowed_tools: vec!["test.tool".to_string()],
                meter_caps: vec![Meter {
                    unit: "usd".to_string(),
                    amount: Quantity::dec("10000", 2).unwrap(), // $100.00
                }],
                rate_limits: None,
                concurrency_limit: None,
                output_mode: None,
                resources: None,
            },
        },
    };
    auth_event.event_id = compute_event_id_for_test(&auth_event);

    // Execution with tokens but no conversion context
    let mut exec_event = ExecutionEvent {
        event_id: make_digest(),
        event_type: "execution".to_string(),
        event_version: "1".to_string(),
        prev_event_id: None,
        occurred_at: make_timestamp(),
        principal_id: make_principal(),
        canonical_profile_id: make_profile(),
        intents: make_intent_anchors(),
        auth_event_id: auth_event.event_id.clone(),
        tool_name: ToolName::parse("test.tool").unwrap(),
        started_at: None,
        ended_at: None,
        meter_used: vec![Meter {
            unit: "tokens.input".to_string(),
            amount: Quantity::int("1000").unwrap(),
        }],
        outcome: Outcome::Success,
        error_code: None,
        output_digest: None,
        output_ref: None,
        resources_touched: None,
        model_id: None,
        provider: None,
        pricing_snapshot_digest: None,
    };
    exec_event.event_id = compute_event_id_for_test(&exec_event);

    let (_, verdict) = verifier
        .verify_execution(&exec_event, &auth_event)
        .unwrap();
    // USD cap exists but no conversion context -> Invalid (missing evidence)
    assert_eq!(verdict, VerificationVerdict::Invalid);
}

#[test]
fn test_missing_cap_for_used_unit() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);

    let mut auth_event = AuthorizationEvent {
        event_id: make_digest(),
        event_type: "authorization".to_string(),
        event_version: "1".to_string(),
        prev_event_id: None,
        occurred_at: make_timestamp(),
        principal_id: make_principal(),
        canonical_profile_id: make_profile(),
        intents: make_intent_anchors(),
        policy_id: "test-policy".to_string(),
        policy_digest: make_digest(),
        decision: Decision::Allow,
        decision_code: "ALLOW".to_string(),
        checks: None,
        hygiene: None,
        authorization: AuthorizationKind::Grant {
            bounds: GrantBounds {
                expires_at: None,
                allowed_tools: vec!["test.tool".to_string()],
                meter_caps: vec![Meter {
                    unit: "tokens.input".to_string(),
                    amount: Quantity::int("1000").unwrap(),
                }],
                rate_limits: None,
                concurrency_limit: None,
                output_mode: None,
                resources: None,
            },
        },
    };
    auth_event.event_id = compute_event_id_for_test(&auth_event);

    // Execution with a different unit not in caps
    let mut exec_event = ExecutionEvent {
        event_id: make_digest(),
        event_type: "execution".to_string(),
        event_version: "1".to_string(),
        prev_event_id: None,
        occurred_at: make_timestamp(),
        principal_id: make_principal(),
        canonical_profile_id: make_profile(),
        intents: make_intent_anchors(),
        auth_event_id: auth_event.event_id.clone(),
        tool_name: ToolName::parse("test.tool").unwrap(),
        started_at: None,
        ended_at: None,
        meter_used: vec![Meter {
            unit: "tokens.output".to_string(), // Different unit
            amount: Quantity::int("500").unwrap(),
        }],
        outcome: Outcome::Success,
        error_code: None,
        output_digest: None,
        output_ref: None,
        resources_touched: None,
        model_id: None,
        provider: None,
        pricing_snapshot_digest: None,
    };
    exec_event.event_id = compute_event_id_for_test(&exec_event);

    let (_, verdict) = verifier
        .verify_execution(&exec_event, &auth_event)
        .unwrap();
    // No USD cap, no direct match -> Ok (optional check skipped)
    assert_eq!(verdict, VerificationVerdict::Ok);
}


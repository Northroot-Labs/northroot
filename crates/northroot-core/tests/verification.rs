use northroot_canonical::{
    Canonicalizer, Digest, DigestAlg, PrincipalId, ProfileId, Quantity, Timestamp, ToolName,
};
use northroot_core::{
    compute_event_id,
    events::{
        AttestationEvent, AuthorizationEvent, AuthorizationKind, CheckpointEvent, Decision,
        ExecutionEvent, GrantBounds, MerkleWindow, Outcome, Signature,
    },
    shared::{IntentAnchors, Meter},
    verification::{ConversionContext, PriceIndexSnapshot, TokenPrice, TokenType, Verifier},
    VerificationVerdict,
};

fn make_canonicalizer() -> Canonicalizer {
    Canonicalizer::new(ProfileId::parse("northroot-canonical-v1").unwrap())
}

fn make_digest() -> Digest {
    Digest::new(
        DigestAlg::Sha256,
        "dGVzdF9kaWdlc3RfZm9yX3Rlc3RpbmdfcHVycG9zZXM",
    )
    .unwrap()
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

fn make_checkpoint_event() -> CheckpointEvent {
    let mut evt = CheckpointEvent {
        event_id: make_digest(), // placeholder
        event_type: "checkpoint".to_string(),
        event_version: "1".to_string(),
        prev_event_id: None,
        occurred_at: make_timestamp(),
        principal_id: make_principal(),
        canonical_profile_id: make_profile(),
        chain_tip_event_id: make_digest(),
        chain_tip_height: 42,
        merkle_root: None,
        window: None,
    };
    evt.event_id = compute_event_id_for_test(&evt);
    evt
}

fn make_attestation_event(checkpoint_event_id: Digest) -> AttestationEvent {
    let mut evt = AttestationEvent {
        event_id: make_digest(), // placeholder
        event_type: "attestation".to_string(),
        event_version: "1".to_string(),
        prev_event_id: None,
        occurred_at: make_timestamp(),
        principal_id: make_principal(),
        canonical_profile_id: make_profile(),
        checkpoint_event_id,
        signatures: vec![Signature {
            alg: "ed25519".to_string(),
            key_id: "did:example:test".to_string(),
            sig: "dGVzdF9zaWduYXR1cmVfYnl0ZXM".to_string(), // base64url-ish, >= 16 chars
        }],
    };
    evt.event_id = compute_event_id_for_test(&evt);
    evt
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
        .verify_execution(&exec_event, &auth_event, None)
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
        .verify_execution(&exec_event, &auth_event, None)
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
        .verify_execution(&exec_event, &auth_event, None)
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
        .verify_execution(&exec_event, &auth_event, None)
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
        .verify_execution(&exec_event, &auth_event, None)
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
        .verify_execution(&exec_event, &auth_event, None)
        .unwrap();
    // No USD cap, no direct match -> Ok (optional check skipped)
    assert_eq!(verdict, VerificationVerdict::Ok);
}

#[test]
fn test_verify_checkpoint_ok_without_merkle_root() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);

    let evt = make_checkpoint_event();
    let (_, verdict) = verifier.verify_checkpoint(&evt).unwrap();
    assert_eq!(verdict, VerificationVerdict::Ok);
}

#[test]
fn test_verify_checkpoint_invalid_merkle_root_without_window() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);

    let mut evt = make_checkpoint_event();
    evt.merkle_root = Some(make_digest());
    evt.window = None;
    evt.event_id = compute_event_id_for_test(&evt);

    let (_, verdict) = verifier.verify_checkpoint(&evt).unwrap();
    assert_eq!(verdict, VerificationVerdict::Invalid);
}

#[test]
fn test_verify_checkpoint_invalid_window_order() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);

    let mut evt = make_checkpoint_event();
    evt.merkle_root = Some(make_digest());
    evt.window = Some(MerkleWindow {
        start_height: Some(10),
        end_height: Some(5),
    });
    evt.event_id = compute_event_id_for_test(&evt);

    let (_, verdict) = verifier.verify_checkpoint(&evt).unwrap();
    assert_eq!(verdict, VerificationVerdict::Invalid);
}

#[test]
fn test_verify_attestation_ok() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);

    let cp = make_checkpoint_event();
    let evt = make_attestation_event(cp.event_id.clone());
    let (_, verdict) = verifier.verify_attestation(&evt).unwrap();
    assert_eq!(verdict, VerificationVerdict::Ok);
}

#[test]
fn test_verify_attestation_invalid_empty_signatures() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);

    let cp = make_checkpoint_event();
    let mut evt = make_attestation_event(cp.event_id.clone());
    evt.signatures = vec![];
    evt.event_id = compute_event_id_for_test(&evt);

    let (_, verdict) = verifier.verify_attestation(&evt).unwrap();
    assert_eq!(verdict, VerificationVerdict::Invalid);
}

#[test]
fn test_verify_attestation_invalid_signature_charset() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);

    let cp = make_checkpoint_event();
    let mut evt = make_attestation_event(cp.event_id.clone());
    evt.signatures[0].sig = "not base64url!".to_string();
    evt.event_id = compute_event_id_for_test(&evt);

    let (_, verdict) = verifier.verify_attestation(&evt).unwrap();
    assert_eq!(verdict, VerificationVerdict::Invalid);
}

#[test]
fn test_pricing_snapshot_digest_validation() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);

    // Create a price index snapshot
    let snapshot = PriceIndexSnapshot {
        as_of: make_timestamp(),
        token_prices: vec![TokenPrice {
            model_id: "gpt-4".to_string(),
            provider: "openai".to_string(),
            token_type: TokenType::Input,
            price_per_token: Quantity::dec("10", 6).unwrap(), // $0.00001 per token
            timestamp: make_timestamp(),
        }],
        compute_rates: None,
        storage_rates: None,
    };

    let conversion_ctx = ConversionContext::new(snapshot);
    let canonicalizer_for_digest = make_canonicalizer();
    let computed_digest = conversion_ctx
        .compute_snapshot_digest(&canonicalizer_for_digest)
        .unwrap();

    // Create authorization event
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
                    amount: Quantity::dec("100", 2).unwrap(), // $1.00 cap
                }],
                rate_limits: None,
                concurrency_limit: None,
                output_mode: None,
                resources: None,
            },
        },
    };
    auth_event.event_id = compute_event_id_for_test(&auth_event);

    // Create execution event with matching digest
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
        model_id: Some("gpt-4".to_string()),
        provider: Some("openai".to_string()),
        pricing_snapshot_digest: Some(computed_digest.clone()),
    };
    exec_event.event_id = compute_event_id_for_test(&exec_event);

    // Verification should pass with matching digest
    let (_, verdict) = verifier
        .verify_execution(&exec_event, &auth_event, Some(&conversion_ctx))
        .unwrap();
    assert_eq!(verdict, VerificationVerdict::Ok);

    // Verification should fail with mismatched digest
    let mut exec_event_bad = exec_event.clone();
    exec_event_bad.pricing_snapshot_digest = Some(make_digest());
    exec_event_bad.event_id = compute_event_id_for_test(&exec_event_bad);

    let (_, verdict) = verifier
        .verify_execution(&exec_event_bad, &auth_event, Some(&conversion_ctx))
        .unwrap();
    assert_eq!(verdict, VerificationVerdict::Invalid);

    // Verification should pass if digest is present but no conversion context
    // (verifier may not have the snapshot, which is acceptable)
    let (_, verdict) = verifier
        .verify_execution(&exec_event, &auth_event, None)
        .unwrap();
    // Note: This will return Invalid due to missing conversion context for USD cap,
    // but the digest validation itself doesn't fail (it's skipped when context is None)
    assert_eq!(verdict, VerificationVerdict::Invalid);
}

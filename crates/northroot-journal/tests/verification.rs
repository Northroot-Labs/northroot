use northroot_canonical::{Canonicalizer, Digest, DigestAlg, ProfileId};
use northroot_core::VerificationVerdict;
use northroot_core::Verifier;
use northroot_journal::{verify_event, verify_event_id};
use serde_json::json;

fn make_canonicalizer() -> Canonicalizer {
    Canonicalizer::new(ProfileId::parse("northroot-canonical-v1").unwrap())
}

fn make_test_authorization_event() -> serde_json::Value {
    // Create a minimal valid authorization event
    // Note: must include meter_caps for grant authorization to be valid
    let mut event = json!({
        "event_type": "authorization",
        "event_version": "1",
        "occurred_at": "2024-01-01T00:00:00Z",
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1",
        "intents": {
            "intent_digest": {
                "alg": "sha-256",
                "b64": "n4bQgYhMfWWaL-qgxVrQHuO5UxN2af8j4V3x8p5Z6Y7"
            }
        },
        "policy_id": "test-policy",
        "policy_digest": {
            "alg": "sha-256",
            "b64": "n4bQgYhMfWWaL-qgxVrQHuO5UxN2af8j4V3x8p5Z6Y7"
        },
        "decision": "allow",
        "decision_code": "ALLOW",
        "authorization": {
            "kind": "grant",
            "bounds": {
                "allowed_tools": ["test.tool"],
                "meter_caps": [{
                    "unit": "usd",
                    "amount": {
                        "t": "dec",
                        "m": "100",
                        "s": 2
                    }
                }]
            }
        }
    });

    // Compute and set event_id
    let canonicalizer = make_canonicalizer();
    let event_id = northroot_core::compute_event_id(&event, &canonicalizer).unwrap();
    event["event_id"] = json!({
        "alg": "sha-256",
        "b64": event_id.b64
    });

    event
}

#[test]
fn test_verify_event_id_valid() {
    let canonicalizer = make_canonicalizer();
    let event = make_test_authorization_event();

    let valid = verify_event_id(&event, &canonicalizer).unwrap();
    assert!(valid);
}

#[test]
fn test_verify_event_id_invalid() {
    let canonicalizer = make_canonicalizer();
    let mut event = make_test_authorization_event();

    // Tamper with event_id
    event["event_id"]["b64"] = json!("tampered");

    let valid = verify_event_id(&event, &canonicalizer).unwrap();
    assert!(!valid);
}

#[test]
fn test_verify_event_authorization() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);
    let event = make_test_authorization_event();

    let verdict = verify_event(&event, &verifier).unwrap();
    assert_eq!(verdict, VerificationVerdict::Ok);
}

#[test]
fn test_verify_event_checkpoint() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);

    let canonicalizer_for_id = make_canonicalizer();
    let mut event = json!({
        "event_type": "checkpoint",
        "event_version": "1",
        "occurred_at": "2024-01-01T00:00:00Z",
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1",
        "chain_tip_event_id": {
            "alg": "sha-256",
            "b64": "n4bQgYhMfWWaL-qgxVrQHuO5UxN2af8j4V3x8p5Z6Y7"
        },
        "chain_tip_height": 1
    });

    let event_id = northroot_core::compute_event_id(&event, &canonicalizer_for_id).unwrap();
    event["event_id"] = json!({
        "alg": "sha-256",
        "b64": event_id.b64
    });

    let verdict = verify_event(&event, &verifier).unwrap();
    assert_eq!(verdict, VerificationVerdict::Ok);
}

#[test]
fn test_verify_event_attestation() {
    let canonicalizer = make_canonicalizer();
    let verifier = Verifier::new(canonicalizer);

    // Use a valid SHA-256 digest (43 base64url chars, no padding)
    // This is the base64url encoding of a 32-byte SHA-256 hash
    let checkpoint_id = Digest::new(
        DigestAlg::Sha256,
        "n4bQgYhMfWWaL-qgxVrQHuO5UxN2af8j4V3x8p5Z6Y7",
    )
    .unwrap();

    let canonicalizer_for_id = make_canonicalizer();
    let mut event = json!({
        "event_type": "attestation",
        "event_version": "1",
        "occurred_at": "2024-01-01T00:00:00Z",
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1",
        "checkpoint_event_id": {
            "alg": "sha-256",
            "b64": checkpoint_id.b64
        },
        "signatures": [{
            "alg": "ed25519",
            "key_id": "did:example:test",
            "sig": "dGVzdF9zaWduYXR1cmVfYnl0ZXM"
        }]
    });

    let event_id = northroot_core::compute_event_id(&event, &canonicalizer_for_id).unwrap();
    event["event_id"] = json!({
        "alg": "sha-256",
        "b64": event_id.b64
    });

    let verdict = verify_event(&event, &verifier).unwrap();
    assert_eq!(verdict, VerificationVerdict::Ok);
}

use northroot_canonical::{compute_event_id, Canonicalizer, ProfileId};
use northroot_journal::{verify_event_id, EventObject};
use serde_json::json;

fn make_canonicalizer() -> Canonicalizer {
    Canonicalizer::new(ProfileId::parse("northroot-canonical-v1").unwrap())
}

fn make_test_event() -> serde_json::Value {
    // Create a minimal valid event
    let mut event = json!({
        "event_type": "test",
        "event_version": "1",
        "occurred_at": "2024-01-01T00:00:00Z",
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1",
        "data": "test data"
    });

    // Compute and set event_id
    let canonicalizer = make_canonicalizer();
    let event_id = compute_event_id(&event, &canonicalizer).unwrap();
    event["event_id"] = json!({
        "alg": "sha-256",
        "b64": event_id.b64
    });

    event
}

#[test]
fn test_verify_event_id_valid() {
    let canonicalizer = make_canonicalizer();
    let event = make_test_event();

    let valid = verify_event_id(&event, &canonicalizer).unwrap();
    assert!(valid);
}

#[test]
fn test_verify_event_id_invalid() {
    let canonicalizer = make_canonicalizer();
    let mut event = make_test_event();

    // Tamper with event_id
    event["event_id"]["b64"] = json!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");

    let valid = verify_event_id(&event, &canonicalizer).unwrap();
    assert!(!valid);
}

#[test]
fn test_verify_event_id_rejects_non_object() {
    let canonicalizer = make_canonicalizer();
    let event = json!(["not", "an", "object"]);

    let err = verify_event_id(&event, &canonicalizer).unwrap_err();
    assert!(err
        .to_string()
        .contains("event payload must be a JSON object"));
}

#[test]
fn test_verify_event_id_rejects_missing_event_id() {
    let canonicalizer = make_canonicalizer();
    let mut event = make_test_event();
    event.as_object_mut().unwrap().remove("event_id");

    let err = verify_event_id(&event, &canonicalizer).unwrap_err();
    assert!(err.to_string().contains("event_id is required"));
}

#[test]
fn test_verify_event_id_rejects_malformed_event_id() {
    let canonicalizer = make_canonicalizer();
    let mut event = make_test_event();
    event["event_id"]["b64"] = json!("not-digest-shaped");

    let err = verify_event_id(&event, &canonicalizer).unwrap_err();
    assert!(err.to_string().contains("event_id must be digest-shaped"));
}

#[test]
fn event_object_wraps_only_structurally_valid_events() {
    let event = make_test_event();
    let event_object = EventObject::try_from(event.clone()).unwrap();

    assert_eq!(event_object.as_json(), &event);
    assert_eq!(
        event_object.claimed_event_id().b64,
        event["event_id"]["b64"].as_str().unwrap()
    );
}

#[test]
fn event_object_does_not_validate_domain_semantics() {
    let canonicalizer = make_canonicalizer();
    let mut event = json!({
        "event_type": "domain-specific-unknown",
        "event_version": "future",
        "arbitrary_payload": {
            "workflow_state": "not-a-kernel-concern"
        }
    });
    let event_id = compute_event_id(&event, &canonicalizer).unwrap();
    event["event_id"] = serde_json::to_value(&event_id).unwrap();

    let event_object = EventObject::try_from(event.clone()).unwrap();

    assert_eq!(event_object.as_json(), &event);
}

use northroot_record::{compute_record_id, validate_record, Record, ValidationError};

fn load_record(json: &str) -> Record {
    serde_json::from_str(json).expect("fixture must deserialize as a record")
}

#[test]
fn valid_event_fixture_has_stable_record_id() {
    let record = load_record(include_str!("testdata/record-v0/valid-event.json"));
    let computed = compute_record_id(&record).expect("record id computes");
    assert_eq!(record.id, computed);
    validate_record(&record).expect("valid event fixture passes core validation");
}

#[test]
fn invalid_uppercase_id_fixture_is_rejected() {
    let record = load_record(include_str!("testdata/record-v0/invalid-uppercase-id.json"));
    assert!(matches!(
        validate_record(&record),
        Err(ValidationError::MalformedContentId { field: "id", .. })
    ));
}

#[test]
fn invalid_profile_id_fixture_is_rejected_before_hash_check() {
    let record = load_record(include_str!("testdata/record-v0/invalid-profile-id.json"));
    assert!(matches!(
        validate_record(&record),
        Err(ValidationError::MalformedProfileId {
            field: "profiles[]",
            ..
        })
    ));
}

#[test]
fn invalid_event_ref_fixture_is_rejected_before_hash_check() {
    let record = load_record(include_str!("testdata/record-v0/invalid-event-ref.json"));
    assert!(matches!(
        validate_record(&record),
        Err(ValidationError::MalformedTypedId {
            field: "refs.causes",
            ..
        })
    ));
}

#[test]
fn invalid_method_ref_fixture_rejects_kind_prefix_mismatch() {
    let record = load_record(include_str!("testdata/record-v0/invalid-method-ref.json"));
    assert!(matches!(
        validate_record(&record),
        Err(ValidationError::TypedRefMismatch {
            field: "context.method.ref",
            expected: "tool",
            ..
        })
    ));
}

#[test]
fn invalid_timestamp_fixture_rejects_impossible_calendar_date() {
    let record = load_record(include_str!("testdata/record-v0/invalid-timestamp.json"));
    assert!(matches!(
        validate_record(&record),
        Err(ValidationError::InvalidTimestamp(value)) if value == "2026-02-30T18:00:00Z"
    ));
}

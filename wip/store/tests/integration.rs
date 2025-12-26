use northroot_canonical::{Digest, DigestAlg, Timestamp};
use northroot_store::{
    executions_for_auth, parse_event, resolve_auth, EventIdFilter, EventJson, EventTypeFilter,
    FilteredReader, JournalBackendReader, JournalBackendWriter, PrincipalFilter, ReadMode,
    StoreReader, StoreWriter, TimeRangeFilter, TypedEvent, WriteOptions,
};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

// Helper to create valid digests for testing (43-44 char base64url)
fn make_digest(id: &str) -> Digest {
    // Pad to valid length (43-44 chars) with base64url-safe chars
    let mut b64 = id.to_string();
    while b64.len() < 43 {
        b64.push('A');
    }
    if b64.len() > 44 {
        b64.truncate(44);
    }
    Digest::new(DigestAlg::Sha256, b64).unwrap()
}

// Helper to create valid digest string for JSON events
fn make_digest_str(id: &str) -> String {
    let mut b64 = id.to_string();
    while b64.len() < 43 {
        b64.push('A');
    }
    if b64.len() > 44 {
        b64.truncate(44);
    }
    b64
}

fn make_test_event(id: &str) -> EventJson {
    json!({
        "event_id": { "alg": "sha-256", "b64": make_digest_str(id) },
        "event_type": "authorization",
        "event_version": "1",
        "occurred_at": "2024-01-01T00:00:00Z",
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1",
        "intents": {
            "intent_digest": { "alg": "sha-256", "b64": "intent" }
        },
        "policy_id": "test-policy",
        "policy_digest": { "alg": "sha-256", "b64": "dGVzdA" },
        "decision": "allow",
        "decision_code": "ALLOW",
        "authorization": {
            "kind": "grant",
            "bounds": {
                "allowed_tools": ["test.tool"],
                "meter_caps": []
            }
        }
    })
}

#[test]
fn test_write_read_round_trip() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Write events
    {
        let mut writer =
            JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("event1")).unwrap();
        writer.append(&make_test_event("event2")).unwrap();
        writer.finish().unwrap();
    }

    // Read events
    {
        let mut reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
        let event1 = reader.read_next().unwrap().unwrap();
        let event2 = reader.read_next().unwrap().unwrap();
        let event3 = reader.read_next().unwrap();

        assert!(event1["event_id"]["b64"]
            .as_str()
            .unwrap()
            .starts_with("event1"));
        assert!(event2["event_id"]["b64"]
            .as_str()
            .unwrap()
            .starts_with("event2"));
        assert!(event3.is_none());
    }
}

#[test]
fn test_payload_too_large() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Create an event with a payload that exceeds 16 MiB
    let mut large_payload = json!({
        "event_id": { "alg": "sha-256", "b64": "large" },
        "event_type": "authorization",
        "event_version": "1",
        "occurred_at": "2024-01-01T00:00:00Z",
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1",
        "intents": {
            "intent_digest": { "alg": "sha-256", "b64": "intent" }
        },
        "policy_id": "test-policy",
        "policy_digest": { "alg": "sha-256", "b64": "dGVzdA" },
        "decision": "allow",
        "decision_code": "ALLOW",
        "authorization": {
            "kind": "grant",
            "bounds": {
                "allowed_tools": ["test.tool"],
                "meter_caps": []
            }
        },
        "large_field": ""
    });

    // Add enough data to exceed 16 MiB
    const TARGET_SIZE: usize = 16 * 1024 * 1024 + 1; // 16 MiB + 1 byte
    let padding_size = TARGET_SIZE - serde_json::to_vec(&large_payload).unwrap().len();
    if let Some(obj) = large_payload.as_object_mut() {
        obj.insert("large_field".to_string(), json!(vec![0u8; padding_size]));
    }

    let mut writer = JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
    let result = writer.append(&large_payload);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        northroot_store::StoreError::PayloadTooLarge
    ));
}

#[test]
fn test_strict_mode_truncation() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Write a complete event
    {
        let mut writer =
            JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("event1")).unwrap();
        writer.finish().unwrap();
    }

    // Truncate the file (simulate partial write)
    let file_size = fs::metadata(&journal_path).unwrap().len();
    let file = fs::OpenOptions::new()
        .write(true)
        .open(&journal_path)
        .unwrap();
    file.set_len(file_size - 5).unwrap();
    drop(file);

    // Strict mode should error
    {
        let mut reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
        assert!(reader.read_next().is_err());
    }
}

#[test]
fn test_permissive_mode_truncation() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Write a complete event
    {
        let mut writer =
            JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("event1")).unwrap();
        writer.finish().unwrap();
    }

    // Truncate the file (simulate partial write)
    let file_size = fs::metadata(&journal_path).unwrap().len();
    let file = fs::OpenOptions::new()
        .write(true)
        .open(&journal_path)
        .unwrap();
    file.set_len(file_size - 5).unwrap();
    drop(file);

    // Permissive mode should handle truncation gracefully
    {
        let mut reader = JournalBackendReader::open(&journal_path, ReadMode::Permissive).unwrap();
        let event = reader.read_next().unwrap();
        // Should return None due to truncation
        assert!(event.is_none());
    }
}

#[test]
fn test_append_to_existing() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Write first event
    {
        let mut writer =
            JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("event1")).unwrap();
        writer.finish().unwrap();
    }

    // Append second event
    {
        let mut writer =
            JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("event2")).unwrap();
        writer.finish().unwrap();
    }

    // Read both events
    {
        let mut reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
        let event1 = reader.read_next().unwrap().unwrap();
        let event2 = reader.read_next().unwrap().unwrap();
        let event3 = reader.read_next().unwrap();

        assert!(event1["event_id"]["b64"]
            .as_str()
            .unwrap()
            .starts_with("event1"));
        assert!(event2["event_id"]["b64"]
            .as_str()
            .unwrap()
            .starts_with("event2"));
        assert!(event3.is_none());
    }
}

#[test]
fn test_flush() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    let mut writer = JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
    writer.append(&make_test_event("event1")).unwrap();
    // Flush should be a no-op but not error
    writer.flush().unwrap();
    writer.finish().unwrap();

    let mut reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
    let event = reader.read_next().unwrap().unwrap();
    assert!(event["event_id"]["b64"]
        .as_str()
        .unwrap()
        .starts_with("event1"));
}

fn make_execution_event(id: &str, auth_id: &str) -> EventJson {
    json!({
        "event_id": { "alg": "sha-256", "b64": make_digest_str(id) },
        "event_type": "execution",
        "event_version": "1",
        "occurred_at": "2024-01-01T01:00:00Z",
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1",
        "intents": {
            "intent_digest": { "alg": "sha-256", "b64": make_digest_str("intent") }
        },
        "auth_event_id": { "alg": "sha-256", "b64": make_digest_str(auth_id) },
        "tool_name": "test.tool",
        "meter_used": [],
        "outcome": "success"
    })
}

fn make_checkpoint_event(id: &str) -> EventJson {
    json!({
        "event_id": { "alg": "sha-256", "b64": make_digest_str(id) },
        "event_type": "checkpoint",
        "event_version": "1",
        "occurred_at": "2024-01-01T02:00:00Z",
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1",
        "chain_tip_event_id": { "alg": "sha-256", "b64": make_digest_str("tip") },
        "chain_tip_height": 1
    })
}

fn make_attestation_event(id: &str, checkpoint_id: &str) -> EventJson {
    json!({
        "event_id": { "alg": "sha-256", "b64": make_digest_str(id) },
        "event_type": "attestation",
        "event_version": "1",
        "occurred_at": "2024-01-01T03:00:00Z",
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1",
        "checkpoint_event_id": { "alg": "sha-256", "b64": make_digest_str(checkpoint_id) },
        "signatures": [{
            "alg": "ed25519",
            "key_id": "key1",
            "sig": "test_signature_base64url_no_padding_here"
        }]
    })
}

#[test]
fn test_parse_event_authorization() {
    let event = make_test_event("auth1");
    let typed = parse_event(&event).unwrap();
    match typed {
        TypedEvent::Authorization(auth) => {
            assert!(auth.event_id.b64.starts_with("auth1"));
            assert_eq!(auth.event_type, "authorization");
        }
        _ => panic!("Expected Authorization event"),
    }
}

#[test]
fn test_parse_event_execution() {
    let event = make_execution_event("exec1", "auth1");
    let typed = parse_event(&event).unwrap();
    match typed {
        TypedEvent::Execution(exec) => {
            assert!(exec.event_id.b64.starts_with("exec1"));
            assert_eq!(exec.event_type, "execution");
        }
        _ => panic!("Expected Execution event"),
    }
}

#[test]
fn test_parse_event_checkpoint() {
    let event = make_checkpoint_event("check1");
    let typed = parse_event(&event).unwrap();
    match typed {
        TypedEvent::Checkpoint(check) => {
            assert!(check.event_id.b64.starts_with("check1"));
            assert_eq!(check.event_type, "checkpoint");
        }
        _ => panic!("Expected Checkpoint event"),
    }
}

#[test]
fn test_parse_event_attestation() {
    let event = make_attestation_event("attest1", "check1");
    let typed = parse_event(&event).unwrap();
    match typed {
        TypedEvent::Attestation(attest) => {
            assert!(attest.event_id.b64.starts_with("attest1"));
            assert_eq!(attest.event_type, "attestation");
        }
        _ => panic!("Expected Attestation event"),
    }
}

#[test]
fn test_parse_event_unknown() {
    let event = json!({
        "event_id": { "alg": "sha-256", "b64": "unknown" },
        "event_type": "unknown_type",
        "event_version": "1"
    });
    let typed = parse_event(&event).unwrap();
    match typed {
        TypedEvent::Unknown(_) => {}
        _ => panic!("Expected Unknown event"),
    }
}

#[test]
fn test_event_type_filter() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    {
        let mut writer =
            JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("auth1")).unwrap();
        writer
            .append(&make_execution_event("exec1", "auth1"))
            .unwrap();
        writer.append(&make_test_event("auth2")).unwrap();
        writer.finish().unwrap();
    }

    let reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
    let filter = EventTypeFilter {
        event_type: "authorization".to_string(),
    };
    let mut filtered = FilteredReader::new(reader, filter);

    let event1 = filtered.read_next().unwrap().unwrap();
    assert!(event1["event_id"]["b64"]
        .as_str()
        .unwrap()
        .starts_with("auth1"));
    let event2 = filtered.read_next().unwrap().unwrap();
    assert!(event2["event_id"]["b64"]
        .as_str()
        .unwrap()
        .starts_with("auth2"));
    assert!(filtered.read_next().unwrap().is_none());
}

#[test]
fn test_principal_filter() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    {
        let mut writer =
            JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("auth1")).unwrap();
        let mut event2 = make_test_event("auth2");
        event2
            .as_object_mut()
            .unwrap()
            .insert("principal_id".to_string(), json!("service:other"));
        writer.append(&event2).unwrap();
        writer.finish().unwrap();
    }

    let reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
    let filter = PrincipalFilter {
        principal_id: "service:test".to_string(),
    };
    let mut filtered = FilteredReader::new(reader, filter);

    let event = filtered.read_next().unwrap().unwrap();
    assert!(event["event_id"]["b64"]
        .as_str()
        .unwrap()
        .starts_with("auth1"));
    assert!(filtered.read_next().unwrap().is_none());
}

#[test]
fn test_time_range_filter() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    {
        let mut writer =
            JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("auth1")).unwrap();
        let mut event2 = make_test_event("auth2");
        event2
            .as_object_mut()
            .unwrap()
            .insert("occurred_at".to_string(), json!("2024-01-01T12:00:00Z"));
        writer.append(&event2).unwrap();
        writer.finish().unwrap();
    }

    let reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
    let filter = TimeRangeFilter {
        after: Some(Timestamp::new("2024-01-01T06:00:00Z".to_string())),
        before: Some(Timestamp::new("2024-01-01T18:00:00Z".to_string())),
    };
    let mut filtered = FilteredReader::new(reader, filter);

    let event = filtered.read_next().unwrap().unwrap();
    assert!(event["event_id"]["b64"]
        .as_str()
        .unwrap()
        .starts_with("auth2"));
    assert!(filtered.read_next().unwrap().is_none());
}

#[test]
fn test_event_id_filter() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    {
        let mut writer =
            JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("auth1")).unwrap();
        writer.append(&make_test_event("auth2")).unwrap();
        writer.finish().unwrap();
    }

    let reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
    let filter = EventIdFilter {
        event_id: make_digest("auth2"),
    };
    let mut filtered = FilteredReader::new(reader, filter);

    let event = filtered.read_next().unwrap().unwrap();
    assert!(event["event_id"]["b64"]
        .as_str()
        .unwrap()
        .starts_with("auth2"));
    assert!(filtered.read_next().unwrap().is_none());
}

#[test]
fn test_filtered_reader_mixed_events() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    {
        let mut writer =
            JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("auth1")).unwrap();
        writer
            .append(&make_execution_event("exec1", "auth1"))
            .unwrap();
        writer.append(&make_checkpoint_event("check1")).unwrap();
        writer
            .append(&make_execution_event("exec2", "auth1"))
            .unwrap();
        writer.finish().unwrap();
    }

    let reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
    let filter = EventTypeFilter {
        event_type: "execution".to_string(),
    };
    let mut filtered = FilteredReader::new(reader, filter);

    let event1 = filtered.read_next().unwrap().unwrap();
    assert!(event1["event_id"]["b64"]
        .as_str()
        .unwrap()
        .starts_with("exec1"));
    let event2 = filtered.read_next().unwrap().unwrap();
    assert!(event2["event_id"]["b64"]
        .as_str()
        .unwrap()
        .starts_with("exec2"));
    assert!(filtered.read_next().unwrap().is_none());
}

#[test]
fn test_resolve_auth() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    {
        let mut writer =
            JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("auth1")).unwrap();
        writer
            .append(&make_execution_event("exec1", "auth1"))
            .unwrap();
        writer.append(&make_test_event("auth2")).unwrap();
        writer.finish().unwrap();
    }

    let mut reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
    let auth_id = make_digest("auth2");
    let auth = resolve_auth(&mut reader, &auth_id).unwrap().unwrap();
    assert!(auth.event_id.b64.starts_with("auth2"));
}

#[test]
fn test_resolve_auth_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    {
        let mut writer =
            JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("auth1")).unwrap();
        writer.finish().unwrap();
    }

    let mut reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
    let auth_id = make_digest("nonexistent");
    let auth = resolve_auth(&mut reader, &auth_id).unwrap();
    assert!(auth.is_none());
}

#[test]
fn test_executions_for_auth() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    {
        let mut writer =
            JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("auth1")).unwrap();
        writer
            .append(&make_execution_event("exec1", "auth1"))
            .unwrap();
        writer.append(&make_test_event("auth2")).unwrap();
        writer
            .append(&make_execution_event("exec2", "auth1"))
            .unwrap();
        writer
            .append(&make_execution_event("exec3", "auth2"))
            .unwrap();
        writer.finish().unwrap();
    }

    let mut reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
    let auth_id = make_digest("auth1");
    let executions = executions_for_auth(&mut reader, &auth_id).unwrap();
    assert_eq!(executions.len(), 2);
    // Extract the prefix (before padding) for comparison
    let exec1_prefix: String = executions[0].event_id.b64.chars().take(5).collect();
    let exec2_prefix: String = executions[1].event_id.b64.chars().take(5).collect();
    assert_eq!(exec1_prefix, "exec1");
    assert_eq!(exec2_prefix, "exec2");
}

#[test]
fn test_executions_for_auth_none() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    {
        let mut writer =
            JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("auth1")).unwrap();
        writer.finish().unwrap();
    }

    let mut reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
    let auth_id = make_digest("auth1");
    let executions = executions_for_auth(&mut reader, &auth_id).unwrap();
    assert_eq!(executions.len(), 0);
}

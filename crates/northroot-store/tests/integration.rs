use northroot_store::{EventJson, JournalBackendReader, JournalBackendWriter, ReadMode, StoreReader, StoreWriter, WriteOptions};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

fn make_test_event(id: &str) -> EventJson {
    json!({
        "event_id": { "alg": "sha-256", "b64": id },
        "event_type": "authorization",
        "event_version": "1",
        "occurred_at": "2024-01-01T00:00:00Z",
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1",
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
        let mut writer = JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
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

        assert_eq!(event1["event_id"]["b64"], "event1");
        assert_eq!(event2["event_id"]["b64"], "event2");
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
    assert!(matches!(result.unwrap_err(), northroot_store::StoreError::PayloadTooLarge));
}

#[test]
fn test_strict_mode_truncation() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Write a complete event
    {
        let mut writer = JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
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
        let mut writer = JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
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
        let mut writer = JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("event1")).unwrap();
        writer.finish().unwrap();
    }

    // Append second event
    {
        let mut writer = JournalBackendWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append(&make_test_event("event2")).unwrap();
        writer.finish().unwrap();
    }

    // Read both events
    {
        let mut reader = JournalBackendReader::open(&journal_path, ReadMode::Strict).unwrap();
        let event1 = reader.read_next().unwrap().unwrap();
        let event2 = reader.read_next().unwrap().unwrap();
        let event3 = reader.read_next().unwrap();

        assert_eq!(event1["event_id"]["b64"], "event1");
        assert_eq!(event2["event_id"]["b64"], "event2");
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
    assert_eq!(event["event_id"]["b64"], "event1");
}


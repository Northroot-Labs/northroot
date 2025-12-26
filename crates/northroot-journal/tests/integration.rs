use northroot_journal::{EventJson, JournalReader, JournalWriter, ReadMode, WriteOptions};
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
        let mut writer = JournalWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append_event(&make_test_event("event1")).unwrap();
        writer.append_event(&make_test_event("event2")).unwrap();
        writer.finish().unwrap();
    }

    // Read events
    {
        let mut reader = JournalReader::open(&journal_path, ReadMode::Strict).unwrap();
        let event1 = reader.read_event().unwrap().unwrap();
        let event2 = reader.read_event().unwrap().unwrap();
        let event3 = reader.read_event().unwrap();

        assert_eq!(event1["event_id"]["b64"], "event1");
        assert_eq!(event2["event_id"]["b64"], "event2");
        assert!(event3.is_none());
    }
}

#[test]
fn test_append_to_existing() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Write first event
    {
        let mut writer = JournalWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append_event(&make_test_event("event1")).unwrap();
        writer.finish().unwrap();
    }

    // Append second event
    {
        let mut writer = JournalWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append_event(&make_test_event("event2")).unwrap();
        writer.finish().unwrap();
    }

    // Read both events
    {
        let mut reader = JournalReader::open(&journal_path, ReadMode::Strict).unwrap();
        let event1 = reader.read_event().unwrap().unwrap();
        let event2 = reader.read_event().unwrap().unwrap();
        let event3 = reader.read_event().unwrap();

        assert_eq!(event1["event_id"]["b64"], "event1");
        assert_eq!(event2["event_id"]["b64"], "event2");
        assert!(event3.is_none());
    }
}

#[test]
fn test_sync_option() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    let options = WriteOptions {
        sync: true,
        ..Default::default()
    };

    let mut writer = JournalWriter::open(&journal_path, options).unwrap();
    writer.append_event(&make_test_event("event1")).unwrap();
    writer.finish().unwrap();

    let mut reader = JournalReader::open(&journal_path, ReadMode::Strict).unwrap();
    let event = reader.read_event().unwrap().unwrap();
    assert_eq!(event["event_id"]["b64"], "event1");
}

#[test]
fn test_permissive_mode_truncation() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Write a complete event
    {
        let mut writer = JournalWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append_event(&make_test_event("event1")).unwrap();
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
        let mut reader = JournalReader::open(&journal_path, ReadMode::Permissive).unwrap();
        let event = reader.read_event().unwrap();
        // Should return None due to truncation
        assert!(event.is_none());
    }

    // Strict mode should error
    {
        let mut reader = JournalReader::open(&journal_path, ReadMode::Strict).unwrap();
        assert!(reader.read_event().is_err());
    }
}

#[test]
fn test_invalid_header_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Write invalid header
    fs::write(&journal_path, b"INVALID HEADER DATA").unwrap();

    assert!(JournalReader::open(&journal_path, ReadMode::Strict).is_err());
}

#[test]
fn test_empty_file_creates_header() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Create empty file
    fs::File::create(&journal_path).unwrap();

    // Opening for write should create header
    {
        let mut writer = JournalWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append_event(&make_test_event("event1")).unwrap();
        writer.finish().unwrap();
    }

    // File should have header + frame
    let file_size = fs::metadata(&journal_path).unwrap().len();
    assert!(file_size > 16); // At least header size

    // Should be readable
    let mut reader = JournalReader::open(&journal_path, ReadMode::Strict).unwrap();
    let event = reader.read_event().unwrap().unwrap();
    assert_eq!(event["event_id"]["b64"], "event1");
}

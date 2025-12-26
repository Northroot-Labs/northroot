use northroot_journal::frame::MAX_PAYLOAD_SIZE;
use northroot_journal::{EventJson, JournalReader, JournalWriter, ReadMode, WriteOptions};
use serde_json::json;
use std::fs;
use std::io::{Seek, Write};
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
        "policy_digest": { "alg": "sha-256", "b64": "n4bQgYhMfWWaL-qgxVrQFaO_TxsrC4Is0V1sFbDwCgg" },
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
fn test_payload_size_limit() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Try to write a payload that exceeds the limit
    let oversized_payload = vec![0u8; MAX_PAYLOAD_SIZE as usize + 1];

    let mut writer = JournalWriter::open(&journal_path, WriteOptions::default()).unwrap();
    let result = writer.append_raw(northroot_journal::FrameKind::EventJson, &oversized_payload);

    assert!(result.is_err());
    match result.unwrap_err() {
        northroot_journal::JournalError::PayloadTooLarge { size, max } => {
            assert_eq!(size, MAX_PAYLOAD_SIZE + 1);
            assert_eq!(max, MAX_PAYLOAD_SIZE);
        }
        _ => panic!("Expected PayloadTooLarge error"),
    }
}

#[test]
fn test_max_payload_size_allowed() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Write a payload at the maximum size
    let max_payload = vec![0u8; MAX_PAYLOAD_SIZE as usize];

    let mut writer = JournalWriter::open(&journal_path, WriteOptions::default()).unwrap();
    writer
        .append_raw(northroot_journal::FrameKind::EventJson, &max_payload)
        .unwrap();
    writer.finish().unwrap();

    // Should be readable
    let mut reader = JournalReader::open(&journal_path, ReadMode::Strict).unwrap();
    let frame = reader.read_frame().unwrap();
    assert!(frame.is_some());
    let (_, payload) = frame.unwrap();
    assert_eq!(payload.len(), MAX_PAYLOAD_SIZE as usize);
}

#[test]
fn test_reserved_bytes_must_be_zero() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Create a valid journal
    {
        let mut writer = JournalWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append_event(&make_test_event("event1")).unwrap();
        writer.finish().unwrap();
    }

    // Corrupt the reserved bytes in the frame header
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&journal_path)
        .unwrap();

    // Seek to frame header (after journal header = 16 bytes)
    file.seek(std::io::SeekFrom::Start(16 + 1)).unwrap(); // kind byte + 1 = first reserved byte
    file.write_all(&[0x01]).unwrap(); // Write non-zero
    drop(file);

    // Reader should reject it
    let mut reader = JournalReader::open(&journal_path, ReadMode::Strict).unwrap();
    assert!(reader.read_frame().is_err());
}

#[test]
fn test_header_reserved_bytes_must_be_zero() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Create a valid journal
    {
        let mut writer = JournalWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append_event(&make_test_event("event1")).unwrap();
        writer.finish().unwrap();
    }

    // Corrupt the reserved bytes in the header
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&journal_path)
        .unwrap();

    file.seek(std::io::SeekFrom::Start(8)).unwrap(); // Start of reserved bytes
    file.write_all(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08])
        .unwrap();
    drop(file);

    // Reader should reject it
    assert!(JournalReader::open(&journal_path, ReadMode::Strict).is_err());
}

#[test]
fn test_partial_write_handling() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Write two complete events
    {
        let mut writer = JournalWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append_event(&make_test_event("event1")).unwrap();
        writer.append_event(&make_test_event("event2")).unwrap();
        writer.finish().unwrap();
    }

    // Get file size and truncate in the middle of the second frame
    let _file_size = fs::metadata(&journal_path).unwrap().len();
    // Find where first event ends by reading it
    let mut reader = JournalReader::open(&journal_path, ReadMode::Strict).unwrap();
    let event1 = reader.read_event().unwrap().unwrap();
    assert_eq!(event1["event_id"]["b64"], "event1");
    let first_event_end = reader.position(); // Position after first event

    // Truncate 10 bytes into the second frame
    let truncate_at = first_event_end + 10;

    let file = fs::OpenOptions::new()
        .write(true)
        .open(&journal_path)
        .unwrap();
    file.set_len(truncate_at).unwrap();
    drop(file);

    // Strict mode should error when trying to read second event
    {
        let mut reader = JournalReader::open(&journal_path, ReadMode::Strict).unwrap();
        // Read first event successfully
        let event1 = reader.read_event().unwrap();
        assert!(event1.is_some());
        // Second read should fail due to truncation
        assert!(reader.read_event().is_err());
    }

    // Permissive mode should handle it gracefully
    {
        let mut reader = JournalReader::open(&journal_path, ReadMode::Permissive).unwrap();
        let event1 = reader.read_event().unwrap();
        assert!(event1.is_some());
        // Second read should return None (EOF due to truncation)
        let event2 = reader.read_event().unwrap();
        assert!(event2.is_none());
    }
}

#[test]
fn test_unknown_frame_kind_skipped() {
    let temp_dir = TempDir::new().unwrap();
    let journal_path = temp_dir.path().join("test.nrj");

    // Write an event
    {
        let mut writer = JournalWriter::open(&journal_path, WriteOptions::default()).unwrap();
        writer.append_event(&make_test_event("event1")).unwrap();
        writer.finish().unwrap();
    }

    // Manually append an unknown frame kind
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&journal_path)
        .unwrap();

    // Write frame header with unknown kind (0xFF)
    let mut frame_header = [0u8; 8];
    frame_header[0] = 0xFF; // Unknown kind
    frame_header[4..8].copy_from_slice(&(10u32.to_le_bytes())); // 10 byte payload
    file.write_all(&frame_header).unwrap();
    file.write_all(b"unknown123").unwrap(); // 10 byte payload
    drop(file);

    // Reader should skip the unknown frame and read the event
    {
        let mut reader = JournalReader::open(&journal_path, ReadMode::Strict).unwrap();
        let event = reader.read_event().unwrap();
        assert!(event.is_some());
        assert_eq!(event.unwrap()["event_id"]["b64"], "event1");

        // Next read should be None (only one event)
        let event2 = reader.read_event().unwrap();
        assert!(event2.is_none());
    }
}

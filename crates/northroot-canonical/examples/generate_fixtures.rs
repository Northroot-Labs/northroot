//! Fixture generator for cross-language testing.
//!
//! Generates golden fixtures for:
//! - Canonicalization (input JSON → canonical bytes)
//! - Event ID computation (event → computed ID)
//! - NRJ journal format (sample .nrj files)
//!
//! Run with: cargo run --example generate_fixtures

use northroot_canonical::{compute_event_id, Canonicalizer, ProfileId};
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

const FIXTURES_DIR: &str = "fixtures";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating fixtures for Northroot 1.0...\n");

    // Create profile and canonicalizer
    let profile = ProfileId::parse("northroot-canonical-v1")?;
    let canonicalizer = Canonicalizer::new(profile);

    // Generate all fixture types
    generate_canonical_fixtures(&canonicalizer)?;
    generate_event_id_fixtures(&canonicalizer)?;
    generate_nrj_fixtures(&canonicalizer)?;

    println!("\n✓ All fixtures generated successfully!");
    Ok(())
}

/// Generate canonical JSON fixtures
fn generate_canonical_fixtures(
    canonicalizer: &Canonicalizer,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new(FIXTURES_DIR).join("canonical");
    fs::create_dir_all(&dir)?;

    println!("Generating canonical fixtures...");

    // Test case 1: Simple object with key reordering
    let simple_object = json!({
        "z": "last",
        "a": "first",
        "m": "middle"
    });
    write_canonical_fixture(&dir, "simple_object", &simple_object, canonicalizer)?;

    // Test case 2: Nested object
    let nested_object = json!({
        "outer": {
            "inner": "value",
            "another": "field"
        },
        "top": "level"
    });
    write_canonical_fixture(&dir, "nested_object", &nested_object, canonicalizer)?;

    // Test case 3: Array (arrays maintain order)
    let with_array = json!({
        "items": ["first", "second", "third"],
        "name": "test"
    });
    write_canonical_fixture(&dir, "with_array", &with_array, canonicalizer)?;

    // Test case 4: Quantity types
    let with_quantities = json!({
        "amount": {
            "t": "dec",
            "m": "12345",
            "s": 2
        },
        "count": {
            "t": "int",
            "v": "42"
        }
    });
    write_canonical_fixture(&dir, "with_quantities", &with_quantities, canonicalizer)?;

    // Test case 5: Unicode and special characters
    let unicode = json!({
        "emoji": "🎉",
        "chinese": "中文",
        "escaped": "line1\nline2\ttab"
    });
    write_canonical_fixture(&dir, "unicode", &unicode, canonicalizer)?;

    // Test case 6: Empty and null values
    let empty_values = json!({
        "empty_string": "",
        "empty_array": [],
        "empty_object": {},
        "null_value": null
    });
    write_canonical_fixture(&dir, "empty_values", &empty_values, canonicalizer)?;

    // Test case 7: Complex nested structure
    let complex = json!({
        "metadata": {
            "version": "1",
            "timestamp": "2024-01-01T00:00:00Z"
        },
        "data": {
            "items": [
                {"id": "1", "value": {"t": "int", "v": "100"}},
                {"id": "2", "value": {"t": "int", "v": "200"}}
            ]
        },
        "checksum": {
            "alg": "sha-256",
            "b64": "n4bQgYhMfWWaL-qgxVrQHuO5UxN2af8j4V3x8p5Z6Y7"
        }
    });
    write_canonical_fixture(&dir, "complex", &complex, canonicalizer)?;

    println!("  ✓ Generated 7 canonical fixtures");
    Ok(())
}

fn write_canonical_fixture(
    dir: &Path,
    name: &str,
    input: &Value,
    canonicalizer: &Canonicalizer,
) -> Result<(), Box<dyn std::error::Error>> {
    // Compute canonical bytes
    let result = canonicalizer.canonicalize(input)?;

    // Write input JSON (pretty-printed for readability)
    let input_path = dir.join(format!("{}_input.json", name));
    let mut input_file = File::create(&input_path)?;
    writeln!(input_file, "{}", serde_json::to_string_pretty(input)?)?;

    // Write expected canonical bytes as hex
    let hex_path = dir.join(format!("{}_canonical.hex", name));
    let mut hex_file = File::create(&hex_path)?;
    writeln!(hex_file, "{}", hex::encode(&result.bytes))?;

    // Write expected canonical bytes as UTF-8 string (for readability)
    let str_path = dir.join(format!("{}_canonical.txt", name));
    let mut str_file = File::create(&str_path)?;
    writeln!(str_file, "{}", String::from_utf8_lossy(&result.bytes))?;

    Ok(())
}

/// Generate event ID fixtures
fn generate_event_id_fixtures(
    canonicalizer: &Canonicalizer,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new(FIXTURES_DIR).join("event-id");
    fs::create_dir_all(&dir)?;

    println!("Generating event ID fixtures...");

    // Test case 1: Minimal event
    let minimal_event = json!({
        "event_type": "test",
        "event_version": "1",
        "occurred_at": "2024-01-01T00:00:00Z",
        "principal_id": "service:test",
        "canonical_profile_id": "northroot-canonical-v1"
    });
    write_event_id_fixture(&dir, "minimal_event", &minimal_event, canonicalizer)?;

    // Test case 2: Checkpoint event (without event_id)
    let checkpoint_event = json!({
        "event_type": "checkpoint",
        "event_version": "1",
        "occurred_at": "2024-01-01T12:00:00Z",
        "principal_id": "service:reconciler",
        "canonical_profile_id": "northroot-canonical-v1",
        "chain_tip_event_id": {
            "alg": "sha-256",
            "b64": "n4bQgYhMfWWaL-qgxVrQHuO5UxN2af8j4V3x8p5Z6Y7"
        },
        "chain_tip_height": 100
    });
    write_event_id_fixture(&dir, "checkpoint_event", &checkpoint_event, canonicalizer)?;

    // Test case 3: Attestation event (without event_id)
    let attestation_event = json!({
        "event_type": "attestation",
        "event_version": "1",
        "occurred_at": "2024-01-01T12:01:00Z",
        "principal_id": "service:attester",
        "canonical_profile_id": "northroot-canonical-v1",
        "checkpoint_event_id": {
            "alg": "sha-256",
            "b64": "Xk9Z8oRmKxNvYwJ3LqPnHgTsUdIfAaZc5VbCyW7M0e1"
        },
        "signatures": [
            {
                "alg": "ed25519",
                "key_id": "did:example:key1",
                "sig": "dGVzdF9zaWduYXR1cmVfYnl0ZXNfaGVyZQ"
            }
        ]
    });
    write_event_id_fixture(&dir, "attestation_event", &attestation_event, canonicalizer)?;

    // Test case 4: Event with optional fields
    let event_with_optionals = json!({
        "event_type": "test",
        "event_version": "1",
        "occurred_at": "2024-06-15T08:30:00Z",
        "principal_id": "service:processor",
        "canonical_profile_id": "northroot-canonical-v1",
        "prev_event_id": {
            "alg": "sha-256",
            "b64": "AbCdEfGhIjKlMnOpQrStUvWxYz0123456789abcdef"
        },
        "metadata": {
            "source": "integration_test",
            "version": "1.0.0"
        }
    });
    write_event_id_fixture(
        &dir,
        "event_with_optionals",
        &event_with_optionals,
        canonicalizer,
    )?;

    println!("  ✓ Generated 4 event ID fixtures");
    Ok(())
}

fn write_event_id_fixture(
    dir: &Path,
    name: &str,
    event: &Value,
    canonicalizer: &Canonicalizer,
) -> Result<(), Box<dyn std::error::Error>> {
    // Compute event ID
    let event_id = compute_event_id(event, canonicalizer)?;

    // Write input event (without event_id)
    let input_path = dir.join(format!("{}_input.json", name));
    let mut input_file = File::create(&input_path)?;
    writeln!(input_file, "{}", serde_json::to_string_pretty(event)?)?;

    // Write computed event_id
    let output_path = dir.join(format!("{}_event_id.json", name));
    let mut output_file = File::create(&output_path)?;
    writeln!(output_file, "{}", serde_json::to_string_pretty(&event_id)?)?;

    // Write full event with event_id included
    let mut full_event = event.clone();
    if let Value::Object(map) = &mut full_event {
        map.insert("event_id".to_string(), serde_json::to_value(&event_id)?);
    }
    let full_path = dir.join(format!("{}_complete.json", name));
    let mut full_file = File::create(&full_path)?;
    writeln!(full_file, "{}", serde_json::to_string_pretty(&full_event)?)?;

    Ok(())
}

/// Generate NRJ journal fixtures
fn generate_nrj_fixtures(canonicalizer: &Canonicalizer) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new(FIXTURES_DIR).join("nrj");
    fs::create_dir_all(&dir)?;

    println!("Generating NRJ journal fixtures...");

    // We need to use the journal writer, but it's in a different crate
    // For now, generate the header manually and document the format

    // NRJ Header: "NRJ1" (4 bytes) + version (2 bytes) + flags (2 bytes) + reserved (8 bytes) = 16 bytes
    let header: [u8; 16] = [
        b'N', b'R', b'J', b'1', // Magic
        0x00, 0x01, // Version 1
        0x00, 0x00, // Flags (reserved)
        0x00, 0x00, 0x00, 0x00, // Reserved
        0x00, 0x00, 0x00, 0x00, // Reserved
    ];

    // Create a sample event
    let sample_event = json!({
        "event_type": "test",
        "event_version": "1",
        "occurred_at": "2024-01-01T00:00:00Z",
        "principal_id": "service:fixture_generator",
        "canonical_profile_id": "northroot-canonical-v1",
        "data": "sample fixture event"
    });
    let event_id = compute_event_id(&sample_event, canonicalizer)?;
    let mut complete_event = sample_event.clone();
    if let Value::Object(map) = &mut complete_event {
        map.insert("event_id".to_string(), serde_json::to_value(&event_id)?);
    }

    // Serialize event to JSON bytes
    let event_bytes = serde_json::to_vec(&complete_event)?;
    let event_len = event_bytes.len() as u32;

    // Create frame: kind (1 byte) + reserved (1 byte) + length (4 bytes) + payload
    let mut frame = Vec::new();
    frame.push(0x01); // FrameKind::EventJson
    frame.push(0x00); // Reserved
    frame.extend_from_slice(&event_len.to_le_bytes());
    frame.extend_from_slice(&event_bytes);

    // Write single-event journal
    let single_path = dir.join("single_event.nrj");
    let mut single_file = File::create(&single_path)?;
    single_file.write_all(&header)?;
    single_file.write_all(&frame)?;

    // Write README describing the fixtures
    let readme_path = dir.join("README.md");
    let mut readme = File::create(&readme_path)?;
    writeln!(readme, "# NRJ Journal Fixtures")?;
    writeln!(readme)?;
    writeln!(
        readme,
        "These fixtures demonstrate the `.nrj` journal format for cross-language testing."
    )?;
    writeln!(readme)?;
    writeln!(readme, "## Format")?;
    writeln!(readme)?;
    writeln!(readme, "### Header (16 bytes)")?;
    writeln!(readme, "- Magic: `NRJ1` (4 bytes)")?;
    writeln!(readme, "- Version: `0x0001` (2 bytes, little-endian)")?;
    writeln!(readme, "- Flags: `0x0000` (2 bytes, reserved)")?;
    writeln!(readme, "- Reserved: 8 bytes of zeros")?;
    writeln!(readme)?;
    writeln!(readme, "### Frame (variable length)")?;
    writeln!(readme, "- Kind: 1 byte (`0x01` = EventJson)")?;
    writeln!(readme, "- Reserved: 1 byte (must be `0x00`)")?;
    writeln!(readme, "- Length: 4 bytes (little-endian, payload size)")?;
    writeln!(readme, "- Payload: `length` bytes of JSON")?;
    writeln!(readme)?;
    writeln!(readme, "## Files")?;
    writeln!(readme)?;
    writeln!(readme, "- `single_event.nrj` - Journal with one test event")?;
    writeln!(readme)?;
    writeln!(readme, "## Verification")?;
    writeln!(readme)?;
    writeln!(readme, "To verify a journal:")?;
    writeln!(readme, "1. Read and validate the 16-byte header")?;
    writeln!(
        readme,
        "2. For each frame: read kind, reserved, length, then payload"
    )?;
    writeln!(readme, "3. Parse payload as JSON")?;
    writeln!(readme, "4. Compute `event_id` from canonical bytes")?;
    writeln!(readme, "5. Verify computed ID matches claimed `event_id`")?;

    // Also write the event JSON for reference
    let event_json_path = dir.join("single_event.json");
    let mut event_json_file = File::create(&event_json_path)?;
    writeln!(
        event_json_file,
        "{}",
        serde_json::to_string_pretty(&complete_event)?
    )?;

    println!("  ✓ Generated 1 NRJ fixture");
    Ok(())
}

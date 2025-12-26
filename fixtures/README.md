# Northroot Golden Fixtures

This directory contains golden test fixtures for cross-language verification of the Northroot protocol.

## Purpose

These fixtures enable any language to verify its implementation against the reference Rust implementation. All values are deterministic and should produce identical results across implementations.

## Directories

### `canonical/`

Canonicalization fixtures demonstrating RFC 8785 (JCS) key ordering and JSON normalization.

Each test case has three files:
- `*_input.json` - Input JSON (may have unordered keys)
- `*_canonical.txt` - Expected canonical UTF-8 output
- `*_canonical.hex` - Expected canonical bytes as hex string

**Test cases:**
- `simple_object` - Basic key reordering
- `nested_object` - Nested structure ordering
- `with_array` - Arrays (order preserved)
- `with_quantities` - Northroot quantity types (Dec, Int)
- `unicode` - Unicode and escape sequences
- `empty_values` - Empty strings, arrays, objects, null
- `complex` - Complex nested structure with quantities

### `event-id/`

Event ID computation fixtures demonstrating the formula:
```
event_id = sha256("northroot:event:v1\0" || canonical_bytes(event - event_id))
```

Each test case has three files:
- `*_input.json` - Event JSON without `event_id` field
- `*_event_id.json` - Computed event ID (Digest object)
- `*_complete.json` - Full event with `event_id` included

**Test cases:**
- `minimal_event` - Simplest possible event
- `checkpoint_event` - Checkpoint governance event
- `attestation_event` - Attestation with signatures
- `event_with_optionals` - Event with optional fields

### `nrj/`

NRJ journal format fixtures demonstrating the binary container format.

- `README.md` - Format specification
- `single_event.nrj` - Binary journal with one event
- `single_event.json` - The event contained in the journal

## Verification Algorithm

### Canonicalization

1. Parse input JSON
2. Recursively sort object keys lexicographically (by Unicode code point)
3. Serialize with no whitespace
4. Compare output bytes with expected canonical bytes

### Event ID

1. Parse event JSON
2. Remove `event_id` field if present
3. Stringify any JSON numbers to strings (hygiene rule)
4. Canonicalize the JSON
5. Compute: `sha256("northroot:event:v1\0" || canonical_bytes)`
6. Base64url-encode (no padding) the 32-byte hash
7. Compare with expected `b64` value

### NRJ Journal

1. Read 16-byte header: `NRJ1` + version + flags + reserved
2. Validate magic = "NRJ1", version = 0x0001, flags = 0x0000
3. Read frames: kind (1) + reserved (1) + length (4) + payload
4. Parse each EventJson frame as JSON
5. Verify each event's `event_id`

## Regenerating Fixtures

From the repository root:

```bash
cargo run --example generate_fixtures -p northroot-canonical
```

## Version

These fixtures are for Northroot 1.0.


# NRJ Journal Fixtures

These fixtures demonstrate the `.nrj` journal format for cross-language testing.

## Format

### Header (16 bytes)
- Magic: `NRJ1` (4 bytes)
- Version: `0x0001` (2 bytes, little-endian)
- Flags: `0x0000` (2 bytes, reserved)
- Reserved: 8 bytes of zeros

### Frame (variable length)
- Kind: 1 byte (`0x01` = EventJson)
- Reserved: 1 byte (must be `0x00`)
- Length: 4 bytes (little-endian, payload size)
- Payload: `length` bytes of JSON

## Files

- `single_event.nrj` - Journal with one test event

## Verification

To verify a journal:
1. Read and validate the 16-byte header
2. For each frame: read kind, reserved, length, then payload
3. Parse payload as JSON
4. Compute `event_id` from canonical bytes
5. Verify computed ID matches claimed `event_id`

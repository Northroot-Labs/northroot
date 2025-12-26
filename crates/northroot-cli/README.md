# northroot-cli

Command-line interface for Northroot event storage and verification.

## Overview

The `northroot` CLI provides a simple interface to interact with Northroot journal files. It supports listing, querying, verifying, and inspecting events stored in append-only journal format.

## Building

From the workspace root:

```bash
cargo build --package northroot-cli --release
```

The binary will be at `target/release/northroot`.

## Commands

### `list` - List events in a journal

List all events in a journal file, optionally filtered.

**Usage:**
```bash
northroot list <journal> [OPTIONS]
```

**Options:**
- `--type <event_type>` - Filter by event type (e.g., "authorization", "execution")
- `--principal <id>` - Filter by principal ID
- `--after <timestamp>` - Filter events after this timestamp (RFC3339 format)
- `--before <timestamp>` - Filter events before this timestamp (RFC3339 format)
- `--json` - Output as JSON (one object per line)

**Examples:**
```bash
# List all events
northroot list events.nrj

# List only execution events
northroot list events.nrj --type execution

# List events for a specific principal
northroot list events.nrj --principal service:api

# List events in a time range
northroot list events.nrj --after 2024-01-01T00:00:00Z --before 2024-01-02T00:00:00Z

# Combine filters
northroot list events.nrj --type execution --principal service:api

# JSON output
northroot list events.nrj --json
```

**Output format (table):**
```
EVENT_ID                                       TYPE            OCCURRED_AT          PRINCIPAL
----------------------------------------------------------------------------------------------------
auth1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA     authorization   2024-01-01T00:00:00Z service:test
exec1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA    execution       2024-01-01T01:00:00Z service:test
```

---

### `get` - Get a single event by ID

Retrieve a specific event by its event ID.

**Usage:**
```bash
northroot get <journal> <event_id>
```

**Arguments:**
- `journal` - Path to journal file
- `event_id` - Event ID (base64url digest, 43-44 characters)

**Examples:**
```bash
# Get an event by ID
northroot get events.nrj auth1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA

# Event ID can be truncated (first 5+ chars) if unique
northroot get events.nrj auth1A
```

**Output:**
JSON object representing the event.

**Exit codes:**
- `0` - Event found
- `1` - Event not found or error

---

### `verify` - Verify all events in a journal

Replay and verify all events, checking integrity and constraints.

**Usage:**
```bash
northroot verify <journal> [OPTIONS]
```

**Options:**
- `--strict` - Exit with error code if any verification fails
- `--json` - Output as JSON array

**Examples:**
```bash
# Verify all events
northroot verify events.nrj

# Strict mode (exit 1 on any failure)
northroot verify events.nrj --strict

# JSON output
northroot verify events.nrj --json
```

**Output format (table):**
```
EVENT_ID                                       TYPE            VERDICT
----------------------------------------------------------------------
auth1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA     authorization   Ok
exec1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA    execution      Ok
```

**Verdicts:**
- `Ok` - All constraints satisfied
- `Denied` - Authorization was denied
- `Violation` - Constraint exceeded (e.g., meter cap)
- `Invalid` - Missing or malformed evidence

**Exit codes:**
- `0` - All verifications passed (or `--strict` not used)
- `1` - Verification failed (only with `--strict`)

---

### `inspect` - Inspect authorization and linked executions

Show an authorization event and all execution events linked to it.

**Usage:**
```bash
northroot inspect <journal> --auth <event_id>
```

**Options:**
- `--auth <event_id>` - Authorization event ID (required)

**Examples:**
```bash
# Inspect an authorization and its executions
northroot inspect events.nrj --auth auth1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
```

**Output:**
JSON object with:
- `authorization` - The authorization event details
- `executions` - Array of linked execution events
- `execution_count` - Number of executions

**Example output:**
```json
{
  "authorization": {
    "event_id": "auth1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "event_type": "authorization",
    "occurred_at": "2024-01-01T00:00:00Z",
    "principal_id": "service:test",
    "decision": "Allow",
    "decision_code": "ALLOW",
    "policy_id": "test-policy"
  },
  "executions": [
    {
      "event_id": "exec1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
      "occurred_at": "2024-01-01T01:00:00Z",
      "tool_name": "test.tool",
      "outcome": "Success",
      "meter_used": []
    }
  ],
  "execution_count": 1
}
```

**Exit codes:**
- `0` - Authorization found
- `1` - Authorization not found

---

### `gen` - Generate a test journal with deterministic events

Generate a journal file with synthetic events for testing and validation.

**Usage:**
```bash
northroot gen --output <path> [OPTIONS]
```

**Options:**
- `--output, -o <path>` - Output journal path (required)
- `--seed <u64>` - Seed for deterministic ID generation (default: 0)
- `--count-auth <n>` - Number of authorization events (default: 2)
- `--count-exec-ok <n>` - Number of valid execution events (default: 2)
- `--count-exec-bad <n>` - Number of orphan/deny execution events (default: 0)
- `--start-ts <timestamp>` - Start timestamp in RFC3339 format (default: "2024-01-01T00:00:00Z")
- `--ts-step-ms <ms>` - Timestamp increment in milliseconds (default: 1000)
- `--with-bad` - Include one malformed record for error testing
- `--force` - Overwrite existing file

**Examples:**
```bash
# Generate a simple journal with 5 auths and 5 executions
northroot gen --output /tmp/test.nrj --count-auth 5 --count-exec-ok 5

# Generate with a specific seed for reproducibility
northroot gen --output test.nrj --seed 42 --count-auth 10

# Generate with error cases for testing verification
northroot gen --output test.nrj --count-auth 2 --count-exec-ok 1 --count-exec-bad 1

# Generate with malformed event to test error handling
northroot gen --output test.nrj --count-auth 1 --with-bad

# Overwrite existing file
northroot gen --output test.nrj --force
```

**Output:**
Prints a summary message: `Generated {n} events to {path}`

**Deterministic behavior:**
- Same seed + counts = byte-identical output
- Event IDs are derived from `sha256(seed || index || type)`
- Timestamps increment by `ts_step_ms` milliseconds per event

**Use cases:**
- Creating test fixtures for integration tests
- Exercising CLI commands with known data
- Testing verification with error cases (bad execs, malformed events)
- Benchmarking with large event counts

---

## Common Workflows

### Verify a journal and check for issues

```bash
# Verify with strict mode
northroot verify events.nrj --strict
```

### Find all executions for a principal

```bash
# List execution events for a principal
northroot list events.nrj --type execution --principal service:api
```

### Inspect a specific authorization chain

```bash
# Get the auth event ID first
AUTH_ID=$(northroot list events.nrj --type authorization --json | jq -r '.[0].event_id.b64')

# Inspect it
northroot inspect events.nrj --auth "$AUTH_ID"
```

### Export events to JSON for processing

```bash
# Export all events as JSON lines
northroot list events.nrj --json > events.jsonl

# Filter and export
northroot list events.nrj --type execution --json > executions.jsonl
```

---

## Help

Get help for any command:

```bash
# General help
northroot --help

# Command-specific help
northroot list --help
northroot get --help
northroot verify --help
northroot inspect --help
```

---

## Error Handling

The CLI provides clear error messages:

- **File not found**: `Failed to open journal: ...`
- **Invalid event ID**: `Invalid event ID: ...` (format validation error)
- **Event not found**: `Event not found` or `Authorization not found`
- **Parse errors**: JSON deserialization errors are shown with context

All commands exit with code `1` on error (except `verify` without `--strict`).

---

## Performance Notes

- **Sequential scans**: All operations perform sequential scans through the journal. For large journals (millions of events), consider building an index layer above the store.
- **Memory usage**: The `verify` command loads all events into memory to resolve authorization linkages. Very large journals may require significant memory.
- **Filtering**: Filters are applied during iteration, so they don't require full scans but still read all events sequentially.

---

## Examples

### Example 1: Basic inspection

```bash
# List all events
$ northroot list events.nrj
EVENT_ID                                       TYPE            OCCURRED_AT          PRINCIPAL
----------------------------------------------------------------------------------------------------
auth1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA     authorization   2024-01-01T00:00:00Z service:test
exec1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA    execution       2024-01-01T01:00:00Z service:test

# Get details of the authorization
$ northroot get events.nrj auth1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
{
  "event_id": { "alg": "sha-256", "b64": "auth1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" },
  "event_type": "authorization",
  ...
}

# Inspect the authorization and its executions
$ northroot inspect events.nrj --auth auth1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
{
  "authorization": { ... },
  "executions": [ ... ],
  "execution_count": 1
}
```

### Example 2: Verification workflow

```bash
# Verify all events
$ northroot verify events.nrj
EVENT_ID                                       TYPE            VERDICT
----------------------------------------------------------------------
auth1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA     authorization   Ok
exec1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA    execution      Ok

# If any fail, use strict mode to exit with error
$ northroot verify events.nrj --strict
# Exits with code 1 if any verdict is not Ok
```

### Example 3: Filtering and export

```bash
# Find all execution events in January 2024
$ northroot list events.nrj \
  --type execution \
  --after 2024-01-01T00:00:00Z \
  --before 2024-01-31T23:59:59Z \
  --json > january_executions.jsonl

# Process with jq
$ cat january_executions.jsonl | jq '.tool_name' | sort | uniq -c
```

---

## Integration with Other Tools

The CLI is designed to work well with standard Unix tools:

```bash
# Pipe to jq for JSON processing
northroot list events.nrj --json | jq '.[] | select(.event_type == "execution")'

# Count events by type
northroot list events.nrj --json | jq -r '.event_type' | sort | uniq -c

# Find events with errors
northroot verify events.nrj --json | jq '.[] | select(.verdict != "Ok")'
```

---

## See Also

- [API Contract](../../docs/developer/api-contract.md) - Core API documentation
- [Store README](../northroot-store/README.md) - Storage abstraction details
- [Journal Format](../../docs/reference/format.md) - Journal file format specification


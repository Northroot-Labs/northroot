# Security Threat Model

**Version:** 1.0  
**Scope:** Early version security assessment for sandboxed sub-process execution  
**Focus:** Journal operations with audit-grade guarantees  
**Date:** 2024

## Executive Summary

**Objective:** Stable CLI contract for sandboxed container execution with verifiable, replayable, offline journal operations.

**Core Principle:** Journal is the primary artifact; agent features deferred.

**Security Posture:** Memory-safe (Rust), no unsafe code, minimal attack surface, deterministic verification.

## Threat Model

### Adversary Capabilities
- Control journal file content (malformed, truncated, oversized)
- Provide malicious CLI arguments (path traversal, resource exhaustion)
- Control environment (sandboxed container with restricted filesystem)
- Observe outputs (stdout/stderr, exit codes)

### Adversary Goals
- Corrupt journal integrity
- Exhaust resources (memory, disk, CPU)
- Escape sandbox via file system access
- Extract sensitive data from journal contents
- Cause denial of service

### Trust Boundaries
- Journal file format (must be validated)
- CLI arguments (must be sanitized)
- File system access (must be restricted to journal files)
- Memory allocations (must be bounded)
- Network access (none - offline verification only)

## Attack Surface

### CLI Interface

#### `list` Command
- **Risk Level:** Low
- **Attack Vectors:**
  - Path traversal via journal path argument
  - Memory exhaustion via large journal files
  - DoS via malformed journal files
- **Mitigations:**
  - Path validation (not currently implemented - HIGH PRIORITY)
  - Streaming reads (already implemented)
  - Strict read mode for malformed files (already implemented)

#### `get` Command
- **Risk Level:** Low
- **Attack Vectors:**
  - Path traversal via journal path
  - Invalid event_id format causing crashes
- **Mitigations:**
  - Path validation needed
  - Event ID validation (already implemented via Digest::new)

#### `verify` Command
- **Risk Level:** Medium
- **Attack Vectors:**
  - Path traversal
  - Memory exhaustion (loads entire journal into memory - CRITICAL)
  - DoS via malformed events
- **Mitigations:**
  - Path validation needed
  - STREAMING VERIFICATION REQUIRED (currently loads all events - HIGH PRIORITY)
  - Strict mode already handles malformed events
- **Critical Issue:** verify command loads entire journal into memory (lines 23-34 in verify.rs)

#### `inspect` Command
- **Risk Level:** Low
- **Attack Vectors:**
  - Path traversal
  - Memory exhaustion (reads journal twice)
- **Mitigations:**
  - Path validation needed
  - Streaming approach (already implemented but reads twice)

#### `gen` Command
- **Risk Level:** High
- **Attack Vectors:**
  - File overwrite (--force flag)
  - Path traversal via output path
  - Resource exhaustion via large event counts
- **Mitigations:**
  - REMOVE FROM PRODUCTION CLI (test utility only)
  - Should be separate tool or hidden behind feature flag
- **Recommendation:** REMOVE for early version

### Common Vulnerabilities

1. **No path validation**
   - **Severity:** High
   - **Location:** All commands accepting file paths
   - **Impact:** Path traversal attacks, access to files outside sandbox
   - **Fix Priority:** Critical

2. **Memory exhaustion in verify command**
   - **Severity:** Critical
   - **Location:** crates/northroot-cli/src/commands/verify.rs:23-34
   - **Impact:** DoS via large journals, violates sandbox resource limits
   - **Fix Priority:** Critical

3. **Unbounded event counts in gen command**
   - **Severity:** Medium
   - **Location:** crates/northroot-cli/src/commands/gen.rs
   - **Impact:** Resource exhaustion
   - **Fix Priority:** Low (remove command)

### Journal Format Security

**Security Properties:**
- Append-only: Enforced by writer (no seek/rewrite)
- Tamper-evident: Event IDs are content-derived hashes
- Framed records: Prevents delimiter ambiguity
- Size limits: 16 MiB per payload (enforced)
- Header validation: Magic bytes, version, flags checked

**Attack Vectors (All Mitigated):**
- Malformed frame headers → Strict validation in RecordFrame::from_bytes
- Oversized payloads → MAX_PAYLOAD_SIZE check (16 MiB)
- Invalid UTF-8 in JSON → UTF-8 validation in reader
- Invalid JSON structure → serde_json parsing with error handling
- Truncated files → ReadMode::Strict vs Permissive

### Memory Safety

**Status:** Excellent
- No unsafe code blocks found
- Rust memory safety guarantees
- Bounded allocations (16 MiB max payload)
- Exception: verify command loads entire journal (needs fix)

### Input Validation

- **Event IDs:** Good - Digest::new validates base64url format and algorithm
- **Timestamps:** Good - Timestamp::parse validates RFC3339 format
- **File Paths:** Poor - None - accepts any string (RISK: Path traversal, symlink attacks) - **FIX REQUIRED**
- **JSON Payloads:** Good - serde_json parsing with error handling, schema validation in core

## What Can Stay

### Core Journal Operations
Essential for audit-grade guarantees:
- **Journal format (northroot-journal):** Keep - Stable, tamper-evident, append-only format
- **CLI read operations (list, get, verify, inspect):** Keep with fixes - Core audit operations
  - Required fixes: Add path validation, fix memory exhaustion in verify (streaming), add resource limits
- **Verification logic (northroot-core):** Keep - Offline, deterministic verification
- **Canonicalization (northroot-canonical):** Keep - Deterministic event identity
- **Store abstraction (northroot-store):** Keep - Pluggable backends, journal is default

### CLI Contract

**Stable Commands:**
- `list` - Enumerate events with filters (sandbox-safe after path validation fix)
- `get` - Retrieve single event by ID (sandbox-safe after path validation fix)
- `verify` - Verify all events in journal (sandbox-safe after streaming fix + path validation)
- `inspect` - Show authorization and linked executions (sandbox-safe after path validation fix)

**Contract Guarantees:**
- Deterministic output for same input
- Offline operation (no network)
- Bounded resource usage (after fixes)
- Stable exit codes (0 = success, 1 = error)
- JSON output when --json flag used
- No side effects (read-only operations)

## What Can Go

### Deferred Features
- **gen command:** Remove - Test utility, not needed for production (risk: file overwrite, resource exhaustion)
- **Agent-related schema fields:** Defer - Agent execution not in scope for early version
- **Write operations via CLI:** Defer - Journal writes should be library API, not CLI
- **Network operations:** None - No network code exists (good)
- **Async I/O:** Defer - Sync I/O sufficient for early version

## Security Hardening Required

### Critical (P0)

1. **Path validation**
   - **Description:** All file path arguments must be validated to prevent path traversal
   - **Location:** All CLI commands
   - **Fix:** Add path normalization and validation, restrict to absolute paths or relative to working directory

2. **Memory exhaustion in verify**
   - **Description:** verify command loads entire journal into memory
   - **Location:** crates/northroot-cli/src/commands/verify.rs:23-34
   - **Fix:** Implement streaming verification, process events one at a time

### High Priority (P1)

1. **Resource limits**
   - **Description:** Add configurable limits for journal size, event count, memory usage
   - **Location:** CLI commands
   - **Fix:** Add --max-events, --max-size flags with safe defaults

2. **Symlink handling**
   - **Description:** Resolve symlinks before opening files
   - **Location:** JournalReader::open, JournalWriter::open
   - **Fix:** Use std::fs::canonicalize or similar, validate result

### Medium Priority (P2)

1. **Error message information leakage**
   - **Description:** Ensure error messages don't leak sensitive paths or data
   - **Location:** All error handling
   - **Fix:** Sanitize paths in error messages, use generic messages for file access errors

2. **Input size limits**
   - **Description:** Validate CLI argument sizes
   - **Location:** Argument parsing
   - **Fix:** Add max length checks for string arguments

## Sandbox Requirements

### Filesystem
- **Read access:** Journal file only (via validated path)
- **Write access:** None (read-only CLI for early version)
- **Restrictions:**
  - No access to parent directories
  - No symlink following (or explicit resolution)
  - No access to /proc, /sys, /dev (except stdin/stdout/stderr)

### Network
- **Access:** None required (offline verification)
- **Restriction:** Block all network access in sandbox

### Memory
- **Limits:** Configurable (default: 512MB recommended)
- **Enforcement:** OS-level limits (ulimit, cgroups)
- **Note:** After streaming fix, memory usage should be O(1) per event

### CPU
- **Limits:** Configurable (default: reasonable timeout)
- **Enforcement:** OS-level limits
- **Note:** Verification is CPU-bound but deterministic

### Capabilities
- **Required:** None (drop all capabilities)
- **User:** Non-root (UID > 0)
- **seccomp:** Restrict syscalls to read/write/exit only

## Audit Guarantees

### Journal Integrity
- **Property:** Tamper-evident
- **Mechanism:** Content-derived event IDs (SHA-256)
- **Verification:** Offline, deterministic
- **Status:** Implemented

### Replayability
- **Property:** Deterministic replay
- **Mechanism:** Append-only format, canonical JSON
- **Verification:** Same input → same output
- **Status:** Implemented

### Durability
- **Property:** Persistent storage
- **Mechanism:** File-based journal, optional fsync
- **Verification:** Journal survives process termination
- **Status:** Implemented

### Offline Verification
- **Property:** No network required
- **Mechanism:** All verification logic is pure
- **Verification:** Works in air-gapped environments
- **Status:** Implemented

### Primitive Operations
- `read_next()` - stream events
- `verify_event()` - verify single event
- `verify_event_id()` - verify event ID matches content
- `read_frame()` - low-level frame access
- **Status:** Implemented
- **Note:** These provide audit-grade building blocks

## Recommendations

### Immediate
- Remove gen command from production CLI
- Add path validation to all file operations
- Implement streaming verification (fix memory exhaustion)
- Add resource limit flags with safe defaults
- Document CLI contract for sandbox execution

### Short Term
- Add integration tests for path traversal prevention
- Add fuzzing for journal format parsing
- Add memory limit tests
- Create sandbox execution guide
- Add structured logging for audit trails

### Long Term
- Consider journal checksums per frame (optional)
- Consider journal compression (optional, v2)
- Consider journal encryption at rest (external to core)
- Consider journal replication/backup tooling (external)

## Compliance Notes

### Audit Requirements
- Journal format supports audit trails
- Event IDs enable content integrity verification
- Offline verification enables air-gapped audits
- Deterministic replay enables forensic analysis

### Gaps
- No built-in journal signing (external tooling needed)
- No built-in journal encryption (external tooling needed)
- No built-in access logging (OS-level needed)
- No built-in retention policies (external tooling needed)

### Mitigation
Core provides primitives; higher layers add policy.


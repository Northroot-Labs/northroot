# Northroot CLI Release Guide

**Status**: Ready for binary release (with append command recommendation)  
**Audience**: Release engineers, Python integration developers  
**Purpose**: Guide for building, testing, and releasing the `northroot` CLI binary for Python integration

---

## Current Status

### ✅ Ready for Release

- **CLI builds successfully** (`northroot-cli` crate)
- **All tests pass** (unit + integration)
- **Production commands available**:
  - `list` - List events in journal
  - `get` - Get event by ID
  - `verify` - Verify all events
  - `inspect` - Inspect authorization chains
- **Security hardening complete**:
  - Path validation and sanitization
  - Resource limits (`--max-events`, `--max-size`)
  - Error sanitization
  - Symlink rejection (optional)

### ⚠️ Production Python Integration Notes

The CLI now provides the `append` command described below, so Python integrations can choose the CLI entry point for both read and write workflows. Existing guidance for using PyO3 bindings remains available if deeper embedding is preferred.

**Current actionable items:**
- `gen` command (dev-tools only) - generates test journals
- `append` command (production) - adds events to journals safely from Python/other callers

## Building the Binary

### Prerequisites

- Rust toolchain 1.91.0+ (see `rust-toolchain.toml`)
- `cargo` (comes with Rust)

### Build Commands

**Production build (recommended):**
```bash
cd /path/to/northroot
cargo build --release --package northroot-cli
```

**Output location:**
```
target/release/northroot
```

**Development build:**
```bash
cargo build --package northroot-cli
# Output: target/debug/northroot
```

**With dev-tools (includes `gen` command):**
```bash
cargo build --release --package northroot-cli --features dev-tools
```

### Verify Build

```bash
# Check binary exists
./target/release/northroot --version  # (if version flag exists)
./target/release/northroot --help

# Run tests
cargo test --package northroot-cli
```

---

## Release Artifacts

### Binary Distribution

**Single binary** (statically linked, no dependencies):
- `northroot` (Linux/macOS/Windows)
- Size: ~2-5 MB (release build)
- No external dependencies required

### Platform-Specific Builds

**Linux (x86_64):**
```bash
cargo build --release --package northroot-cli --target x86_64-unknown-linux-gnu
```

**macOS (Apple Silicon):**
```bash
cargo build --release --package northroot-cli --target aarch64-apple-darwin
```

**macOS (Intel):**
```bash
cargo build --release --package northroot-cli --target x86_64-apple-darwin
```

**Windows:**
```bash
cargo build --release --package northroot-cli --target x86_64-pc-windows-msvc
```

### Release Package Structure

```
northroot-v0.1.0/
├── README.md
├── LICENSE-APACHE
├── LICENSE-MIT
├── bin/
│   ├── northroot-linux-x86_64
│   ├── northroot-macos-aarch64
│   ├── northroot-macos-x86_64
│   └── northroot-windows-x86_64.exe
└── docs/
    └── CLI_README.md
```

---

## Testing Before Release

### Unit Tests

```bash
cargo test --package northroot-cli
```

**Expected output:**
- All path validation tests pass
- All integration tests pass (7 tests)
- No warnings or errors

### Integration Tests

```bash
# Run full test suite
cargo test --all --all-features

# Run specific integration test
cargo test --package northroot-cli --test integration
```

### Manual Testing Checklist

- [ ] `northroot list <journal>` - Lists events correctly
- [ ] `northroot get <journal> <event_id>` - Retrieves event
- [ ] `northroot verify <journal>` - Verifies all events
- [ ] `northroot inspect <journal> --auth <id>` - Inspects authorization
- [ ] Path validation rejects traversal (`../`)
- [ ] Path validation rejects symlinks (if enabled)
- [ ] Resource limits work (`--max-events`, `--max-size`)
- [ ] JSON output format is valid
- [ ] Error messages are sanitized (no absolute paths)

---

## Python Integration

### Current State: Read-Only Operations

The CLI currently supports **read-only** operations from Python:

```python
import subprocess
import json
from pathlib import Path

class NorthrootCLI:
    def __init__(self, binary_path: str = "northroot"):
        self.binary = binary_path
    
    def list_events(self, journal_path: Path, filters: dict = None) -> list:
        """List events in journal."""
        cmd = [self.binary, "list", str(journal_path), "--json"]
        # Add filters if provided
        result = subprocess.run(cmd, capture_output=True, check=True, text=True)
        events = [json.loads(line) for line in result.stdout.strip().split('\n') if line]
        return events
    
    def get_event(self, journal_path: Path, event_id: str) -> dict:
        """Get event by ID."""
        cmd = [self.binary, "get", str(journal_path), event_id]
        result = subprocess.run(cmd, capture_output=True, check=True, text=True)
        return json.loads(result.stdout)
    
    def verify_journal(self, journal_path: Path, strict: bool = False) -> dict:
        """Verify all events in journal."""
        cmd = [self.binary, "verify", str(journal_path), "--json"]
        if strict:
            cmd.append("--strict")
        result = subprocess.run(cmd, capture_output=True, check=True, text=True)
        return json.loads(result.stdout)
    
    def inspect_auth(self, journal_path: Path, auth_id: str) -> dict:
        """Inspect authorization and linked executions."""
        cmd = [self.binary, "inspect", str(journal_path), "--auth", auth_id]
        result = subprocess.run(cmd, capture_output=True, check=True, text=True)
        return json.loads(result.stdout)
```

### Append Command

The new CLI `append` command is the recommended way to record events from Python integrations or automation scripts without needing direct FFI exposure.

**Usage highlights:**
- `northroot append <journal>` appends an event JSON provided via `--event` or stdin (`--stdin`).
- Paths are validated and sanitized before any write happens, reusing `path::validate_journal_path`.
- The command enforces payload limits through `northroot-store` and prints success/error diagnostics to stdout/stderr.

**Python snippet:**
```python
def append_event(self, journal_path: Path, event: dict) -> bool:
    event_json = json.dumps(event)
    cmd = [self.binary, "append", str(journal_path), "--stdin"]
    result = subprocess.run(cmd, input=event_json, capture_output=True, check=True, text=True)
    return result.returncode == 0
```

**Alternative**: Use the PyO3 bindings if you need tighter integration.

----

## Release Process

### 1. Pre-Release Checklist

- [ ] All tests pass (`cargo test --all --all-features`)
- [ ] No clippy warnings (`cargo clippy --all-targets --all-features -- -D warnings`)
- [ ] Formatting is correct (`cargo fmt --all --check`)
- [ ] Release readiness script (`./scripts/release-check.sh`) passes
- [ ] Version number updated in `Cargo.toml`
- [ ] CHANGELOG.md updated (if exists)
- [ ] Documentation reviewed

### 2. Build Release Binaries

```bash
# Set version
VERSION="0.1.0"

# Create release directory
mkdir -p release/northroot-v${VERSION}
cd release/northroot-v${VERSION}

# Build for each platform
# Linux
cargo build --release --package northroot-cli --target x86_64-unknown-linux-gnu
cp ../../target/x86_64-unknown-linux-gnu/release/northroot bin/northroot-linux-x86_64

# macOS (Apple Silicon)
cargo build --release --package northroot-cli --target aarch64-apple-darwin
cp ../../target/aarch64-apple-darwin/release/northroot bin/northroot-macos-aarch64

# macOS (Intel)
cargo build --release --package northroot-cli --target x86_64-apple-darwin
cp ../../target/x86_64-apple-darwin/release/northroot bin/northroot-macos-x86_64

# Windows
cargo build --release --package northroot-cli --target x86_64-pc-windows-msvc
cp ../../target/x86_64-pc-windows-msvc/release/northroot.exe bin/northroot-windows-x86_64.exe
```

### 3. Create Release Package

```bash
# Copy documentation
cp ../../README.md .
cp ../../LICENSE-APACHE .
cp ../../LICENSE-MIT .
cp ../../crates/northroot-cli/README.md docs/CLI_README.md

# Create tarball/zip
tar -czf northroot-v${VERSION}.tar.gz northroot-v${VERSION}/
# or
zip -r northroot-v${VERSION}.zip northroot-v${VERSION}/
```

### 4. Verify Release Artifacts

- [ ] All binaries execute (`--help` works)
- [ ] Binaries are correct architecture
- [ ] Documentation is included
- [ ] Licenses are included
- [ ] Checksums generated (SHA256)

### 5. Release Notes Template

```markdown
# Northroot CLI v0.1.0

## Features
- List events in journal files
- Get events by ID
- Verify all events in journal
- Inspect authorization chains
- Path validation and security hardening
- Resource limits for sandboxed environments

## Installation
Download the binary for your platform and place in PATH.

## Python Integration
See RELEASE_GUIDE.md for Python integration examples.

## Breaking Changes
None (initial release)

## Security
- Path validation prevents directory traversal
- Resource limits prevent DoS
- Error sanitization prevents path leakage
```

---

## Distribution Options

### Option 1: GitHub Releases

1. Create release tag: `git tag v0.1.0`
2. Push tag: `git push origin v0.1.0`
3. Create GitHub release with binaries attached
4. Add release notes

### Option 2: Package Managers

**Homebrew (macOS):**
```ruby
# Formula: northroot.rb
class Northroot < Formula
  desc "Northroot event storage and verification CLI"
  homepage "https://github.com/yourorg/northroot"
  url "https://github.com/yourorg/northroot/releases/download/v0.1.0/northroot-v0.1.0.tar.gz"
  sha256 "..."
  
  def install
    bin.install "bin/northroot-macos-aarch64" => "northroot"
  end
end
```

**Cargo Install (Rust users):**
```bash
cargo install --path crates/northroot-cli
```

### Option 3: Docker

```dockerfile
FROM rust:1.91 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --package northroot-cli

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/northroot /usr/local/bin/northroot
ENTRYPOINT ["northroot"]
```

---

## Post-Release

### Monitoring

- Monitor for issues in Python integrations
- Track binary download/usage
- Collect feedback on missing features (e.g., `append` command)

### Next Steps

1. **Document append command usage and CLI script** for integrations
2. **Create PyO3 bindings** as alternative integration method
3. **Add version flag** (`northroot --version`)
4. **Add progress indicators** for large journal operations
5. **Add index support** for faster lookups in large journals

---

## Troubleshooting

### Build Issues

**Error: "could not find `Cargo.toml`"**
- Ensure you're in the workspace root
- Check `Cargo.toml` exists at root

**Error: "toolchain not found"**
- Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Or use `rustup toolchain install 1.91.0`

### Runtime Issues

**Binary not found**
- Ensure binary is in PATH
- Or use full path: `/path/to/northroot --help`

**Permission denied**
- Make binary executable: `chmod +x northroot`

**Journal file errors**
- Check file permissions
- Verify journal format (should start with `NRJ1` magic)

---

## References

- [CLI README](crates/northroot-cli/README.md) - Full CLI documentation
- [Journal Format](docs/reference/format.md) - Journal file specification
- [Handoff Document](docs/operator/journal-cli-handoff.md) - Security context
- [Core Invariants](CORE_INVARIANTS.md) - Design principles

---

*Last updated: December 2025*

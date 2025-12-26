# Northroot CLI Release Guide

**Status**: v1.0.0 - Stable API, ready for production use  
**Audience**: Release engineers, Python integration developers  
**Purpose**: Guide for building, testing, and releasing the `northroot` CLI binary for Python integration

---

## Current Status

### ✅ v1.0.0 - Stable Release

**Version**: All crates at `1.0.0` (stable API contract)  
**API Status**: Stable (see [API Contract](docs/developer/api-contract.md))  
**Last Tag**: `v0.1.0` (legacy, current codebase is 1.0.0)

- **CLI builds successfully** (`northroot` package in `apps/northroot/`)
- **All tests pass** (unit + integration + golden tests)
- **Production commands available**:
  - `canonicalize` - Show canonical bytes for input JSON
  - `event-id` - Compute event_id for input JSON
  - `list` - List events in journal
  - `verify` - Verify all event IDs in journal
- **Security hardening complete**:
  - Path validation and sanitization
  - Resource limits (`--max-events`, `--max-size`)
  - Error sanitization
  - Symlink rejection (optional)
- **Workspace structure**: CLI intentionally **not** in workspace (by design, see [AGENTS.md](AGENTS.md))

### ⚠️ Future Commands (Not Yet Implemented)

The following commands are planned but not yet implemented:
- `get` - Get event by ID
- `inspect` - Inspect authorization chains
- `append` - Add events to journals (for Python/automation integration)
- `gen` - Generate test journals (dev-tools only)

---

## Automated Release Workflow

Northroot uses a **tag-driven release workflow** that builds and publishes binaries when a `v*` tag is pushed.

### Label Taxonomy

| Label | Semver Bump | Use Case | Release? |
|-------|-------------|----------|----------|
| `release:patch` | 1.0.X → 1.0.X+1 | Bug fix, security patch, doc fix | Yes |
| `release:minor` | 1.X.0 → 1.X+1.0 | New feature, additive API | Yes |
| `release:major` | X.0.0 → X+1.0.0 | Breaking API change, contract change | Yes |
| `chore` | - | Deps, CI, tooling changes | No |
| `style` | - | Formatting, naming, whitespace | No |
| `contract` | flag | API surface changes (review flag) | No (must pair with release label) |

**Note**: Labels are for review/intent and changelog hygiene. **Releases are created from tags**, not directly from labels.

### Release Flow

1. **Open PR** with your changes
2. **Apply label** (`release:patch`, `release:minor`, `release:major`, or `chore`/`style` for no release)
3. **CI runs** (formatting, clippy, tests) - gates the PR
4. **Merge to main**
5. **Bump versions in the same PR** (or a dedicated version PR) as needed:
   - Workspace crates: `cargo set-version <VERSION> --workspace`
   - CLI app (not in workspace): `cd apps/northroot && cargo set-version <VERSION>`
6. **Create a signed release tag** on the merged commit (recommended):
   - `git tag -s v<VERSION> -m "Release v<VERSION>"`
   - `git push origin v<VERSION>`
7. GitHub Actions builds binaries and publishes the GitHub Release for that tag.

**Important**: The CLI app (`apps/northroot`) is **not** in the workspace by design. The release workflow handles this correctly by using `--manifest-path` for builds and explicitly bumping the CLI version separately.

### Contract Change Handling

The `contract` label flags API surface changes for review. It does **not** trigger a release alone but signals that the change requires extra scrutiny. Must be paired with a release label (`release:patch`, `release:minor`, or `release:major`) for actual release.

**Paths that auto-suggest `contract` label:**
- `docs/developer/api-contract.md`
- `schemas/**`
- `crates/*/src/lib.rs` (public API exports)
- `apps/northroot/src/main.rs` (CLI command interface)

### Manual Release (Fallback)

If you need to release manually or the automated workflow fails:

```bash
# 1. Bump versions (ideally via PR to main)
VERSION="1.0.1"
cargo set-version "$VERSION" --workspace
cd apps/northroot && cargo set-version "$VERSION" && cd ../..

# 2. Commit (via PR) and merge to main
git add crates/*/Cargo.toml apps/northroot/Cargo.toml
git commit -m "chore: bump version to $VERSION"

# 3. Create a signed tag on the merge commit and push it
git tag -s "v$VERSION" -m "Release v$VERSION"
git push origin "v$VERSION"

# 4. GitHub Actions will build binaries + publish the GitHub Release for v$VERSION
```

---

## Building the Binary

### Prerequisites

- Rust toolchain 1.91.0+ (see `rust-toolchain.toml`)
- `cargo` (comes with Rust)

### Build Commands

**Production build (recommended):**
```bash
cd /path/to/northroot/apps/northroot
cargo build --release
```

Or from workspace root:
```bash
cd /path/to/northroot
cargo build --release --manifest-path apps/northroot/Cargo.toml
```

**Output location:**
```
apps/northroot/target/release/northroot
```

**Development build:**
```bash
cd apps/northroot
cargo build
# Output: apps/northroot/target/debug/northroot
```

### Verify Build

```bash
# Check binary exists
./target/release/northroot --version  # (if version flag exists)
./target/release/northroot --help

# Run tests
cd apps/northroot
cargo test
# Or from workspace root:
# cargo test --manifest-path apps/northroot/Cargo.toml
```

---

## Branching Strategy

### Recommended Workflow

**For all changes (major, minor, patch):**
1. Create feature branch from `main`: `git checkout -b feature/description`
2. Make changes, commit with conventional commits
3. Open PR to `main`
4. Apply appropriate label (`release:major`, `release:minor`, `release:patch`, or `chore`/`style`)
5. After merge, automated release workflow handles versioning and tagging

**Branch naming conventions:**
- `feature/description` - New features, enhancements
- `fix/description` - Bug fixes
- `docs/description` - Documentation updates
- `chore/description` - Tooling, CI, dependencies

**No separate release branches needed** - the automated workflow handles releases on merge to `main`.

### Version Bump Guidelines

- **`release:patch`** (1.0.X → 1.0.X+1):
  - Bug fixes
  - Security patches
  - Documentation corrections
  - Non-breaking dependency updates

- **`release:minor`** (1.X.0 → 1.X+1.0):
  - New commands or features
  - Additive API changes (new optional fields, new functions)
  - New CLI flags (backward compatible)
  - Performance improvements

- **`release:major`** (X.0.0 → X+1.0.0):
  - Breaking API changes
  - Canonicalization rule changes
  - Schema changes that break compatibility
  - Removal of public APIs
  - Breaking CLI command changes

- **`chore`/`style`** (no version bump):
  - CI/CD changes
  - Code formatting
  - Internal refactoring (no API changes)
  - Dependency updates (non-breaking)

### Workspace Decision: CLI Not in Workspace

The CLI app (`apps/northroot`) is **intentionally not in the workspace** for these reasons:

1. **Separation of concerns**: CLI is an application, not a library crate
2. **Independent versioning**: CLI may evolve differently than core crates
3. **Build isolation**: CLI builds don't need workspace-wide features
4. **Deployment flexibility**: CLI can be built independently for distribution

The release workflow correctly handles this by:
- Using `--manifest-path apps/northroot/Cargo.toml` for builds
- Explicitly bumping CLI version separately from workspace crates
- Staging both workspace and CLI `Cargo.toml` files in git

**Do not add CLI to workspace** - this is by design.

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
cd apps/northroot
cargo build --release --target x86_64-unknown-linux-gnu
```

**macOS (Apple Silicon):**
```bash
cd apps/northroot
cargo build --release --target aarch64-apple-darwin
```

**macOS (Intel):**
```bash
cd apps/northroot
cargo build --release --target x86_64-apple-darwin
```

**Windows:**
```bash
cd apps/northroot
cargo build --release --target x86_64-pc-windows-msvc
```

### Release Package Structure

```
northroot-v1.0.0/
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
cd apps/northroot
cargo test
```

**Expected output:**
- All path validation tests pass
- All integration tests pass
- No warnings or errors

### Integration Tests

```bash
# Run full test suite (from workspace root)
cargo test --all --all-features

# Run journal integration tests
cargo test --package northroot-journal --test integration
```

### Manual Testing Checklist

- [ ] `northroot canonicalize <input>` - Produces canonical bytes
- [ ] `northroot event-id <input>` - Computes event_id correctly
- [ ] `northroot list <journal>` - Lists events correctly
- [ ] `northroot verify <journal>` - Verifies all events
- [ ] Path validation rejects traversal (`../`)
- [ ] Path validation rejects symlinks (if enabled)
- [ ] Resource limits work (`--max-events`, `--max-size`)
- [ ] JSON output format is valid (when `--json` flag used)
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
    
    def canonicalize(self, input_json: str) -> str:
        """Canonicalize JSON input."""
        cmd = [self.binary, "canonicalize"]
        result = subprocess.run(cmd, input=input_json, capture_output=True, check=True, text=True)
        return result.stdout.strip()
    
    def compute_event_id(self, input_json: str) -> dict:
        """Compute event_id for JSON input."""
        cmd = [self.binary, "event-id"]
        result = subprocess.run(cmd, input=input_json, capture_output=True, check=True, text=True)
        return json.loads(result.stdout)
    
    def list_events(self, journal_path: Path, max_events: int = None, max_size: int = None) -> list:
        """List events in journal."""
        cmd = [self.binary, "list", str(journal_path), "--json"]
        if max_events:
            cmd.extend(["--max-events", str(max_events)])
        if max_size:
            cmd.extend(["--max-size", str(max_size)])
        result = subprocess.run(cmd, capture_output=True, check=True, text=True)
        events = [json.loads(line) for line in result.stdout.strip().split('\n') if line]
        return events
    
    def verify_journal(self, journal_path: Path, strict: bool = False, max_events: int = None, max_size: int = None) -> dict:
        """Verify all events in journal."""
        cmd = [self.binary, "verify", str(journal_path), "--json"]
        if strict:
            cmd.append("--strict")
        if max_events:
            cmd.extend(["--max-events", str(max_events)])
        if max_size:
            cmd.extend(["--max-size", str(max_size)])
        result = subprocess.run(cmd, capture_output=True, check=True, text=True)
        return json.loads(result.stdout)
```

### Future: Write Operations

The `append` command is planned but not yet implemented. For now, write operations should use the Rust crates directly (`northroot-canonical` and `northroot-journal`) or PyO3 bindings if available.

----

## Release Process

> **Note**: The release process is now **automated via GitHub Actions** when PRs with release labels are merged. The manual process below is a fallback for edge cases or manual releases.

### Automated Release (Recommended)

See "Automated Release Workflow" section above. Simply label your PR and merge.

### Manual Release (Fallback)

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
VERSION="1.0.0"

# Create release directory
mkdir -p release/northroot-v${VERSION}
cd release/northroot-v${VERSION}

# Build for each platform (from apps/northroot directory)
cd apps/northroot

# Linux
cargo build --release --target x86_64-unknown-linux-gnu
cp ../../target/x86_64-unknown-linux-gnu/release/northroot ../../release/northroot-v${VERSION}/bin/northroot-linux-x86_64

# macOS (Apple Silicon)
cargo build --release --target aarch64-apple-darwin
cp ../../target/aarch64-apple-darwin/release/northroot ../../release/northroot-v${VERSION}/bin/northroot-macos-aarch64

# macOS (Intel)
cargo build --release --target x86_64-apple-darwin
cp ../../target/x86_64-apple-darwin/release/northroot ../../release/northroot-v${VERSION}/bin/northroot-macos-x86_64

# Windows
cargo build --release --target x86_64-pc-windows-msvc
cp ../../target/x86_64-pc-windows-msvc/release/northroot.exe ../../release/northroot-v${VERSION}/bin/northroot-windows-x86_64.exe
```

### 3. Create Release Package

```bash
# Copy documentation
cp ../../README.md .
cp ../../LICENSE-APACHE .
cp ../../LICENSE-MIT .
# Note: CLI README may not exist yet

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
# Northroot CLI v1.0.0

## Features
- Canonicalize JSON input (RFC 8785 + Northroot rules)
- Compute event_id for JSON events
- List events in journal files
- Verify all event IDs in journal
- Path validation and security hardening
- Resource limits for sandboxed environments

## Installation
Download the binary for your platform and place in PATH.

## Python Integration
See RELEASE_GUIDE.md for Python integration examples.

## Breaking Changes
None (stable 1.0.0 release)

## Security
- Path validation prevents directory traversal
- Resource limits prevent DoS
- Error sanitization prevents path leakage

## API Stability
This release marks the stable 1.0.0 API. See [API Contract](docs/developer/api-contract.md) for stability guarantees.
```

---

## Distribution Options

### Option 1: GitHub Releases

1. Create release tag: `git tag v1.0.0`
2. Push tag: `git push origin v1.0.0`
3. Create GitHub release with binaries attached
4. Add release notes

### Option 2: Package Managers

**Homebrew (macOS):**
```ruby
# Formula: northroot.rb
class Northroot < Formula
  desc "Northroot event storage and verification CLI"
  homepage "https://github.com/yourorg/northroot"
  url "https://github.com/yourorg/northroot/releases/download/v1.0.0/northroot-v1.0.0.tar.gz"
  sha256 "..."
  
  def install
    bin.install "bin/northroot-macos-aarch64" => "northroot"
  end
end
```

**Cargo Install (Rust users):**
```bash
cargo install --path apps/northroot
# Or from workspace root:
# cargo install --manifest-path apps/northroot/Cargo.toml
```

### Option 3: Docker

```dockerfile
FROM rust:1.91 as builder
WORKDIR /app
COPY . .
RUN cd apps/northroot && cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/apps/northroot/target/release/northroot /usr/local/bin/northroot
ENTRYPOINT ["northroot"]
```

---

## Post-Release

### Monitoring

- Monitor for issues in Python integrations
- Track binary download/usage
- Collect feedback on missing features (e.g., `append` command)

### Next Steps

1. **Implement `append` command** for write operations from Python/automation
2. **Implement `get` command** for retrieving events by ID
3. **Implement `inspect` command** for authorization chain inspection
4. **Add version flag** (`northroot --version`)
5. **Create PyO3 bindings** as alternative integration method
6. **Add progress indicators** for large journal operations

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

- [API Contract](docs/developer/api-contract.md) - Public API surface
- [Journal Format](docs/reference/format.md) - Journal file specification
- [Core Invariants](CORE_INVARIANTS.md) - Design principles
- [GOVERNANCE.md](GOVERNANCE.md) - Project constitution

---

*Last updated: December 2025*

# Quality & Testing Harness
# Run `just` to see all available commands

# Fast pre-push checks
fmt:
    cargo fmt --all --check

lint:
    cargo clippy --all-targets --all-features -- -D warnings

test:
    cargo test --all --all-features

golden:
    cargo test --package northroot-canonical --test golden

schema:
    python3 scripts/validate_schemas.py

cli-test:
    cargo test --manifest-path apps/northroot/Cargo.toml

install-hooks:
    bash scripts/install_git_hooks.sh

codex-verify:
    bash scripts/codex_verify.sh

fetch-refs:
    bash scripts/fetch_remote_refs.sh

sync-check:
    bash scripts/check_worktree_sync.sh

sync-main:
    bash scripts/safe_sync_main.sh

# Combined fast QA suite
qa: fmt lint test golden schema

# Coverage (requires cargo-llvm-cov)
coverage:
    cargo llvm-cov --workspace --ignore-filename-regex '(/tests?/|/examples?/)' --lcov --output-path lcov.info
    cargo llvm-cov --ignore-filename-regex '(/tests?/|/examples?/)' report --html --output-dir coverage

# Security audits
audit:
    cargo deny check
    cargo audit

# Miri (UB detection, nightly only)
miri:
    cargo +nightly miri test --package northroot-canonical
    cargo +nightly miri test --package northroot-journal

# Fuzzing (requires cargo-fuzz)
fuzz target:
    cd crates/northroot-canonical && cargo fuzz run {{target}} -- -max_total_time=60

# Documentation
docs:
    cargo doc --workspace --no-deps

# Documentation with doctests
docs-test:
    cargo test --workspace --doc

# Full nightly suite (slow)
nightly: fmt lint test golden docs coverage audit

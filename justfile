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

# Combined fast QA suite
qa: fmt lint test golden

# Coverage (requires cargo-llvm-cov)
coverage:
    cargo llvm-cov --workspace --ignore-filename-regex '(/tests?/|/examples?/)' --lcov --output-path lcov.info
    cargo llvm-cov --workspace --ignore-filename-regex '(/tests?/|/examples?/)' report --html --output-dir coverage

# Security audits
audit:
    cargo deny check
    cargo audit

# Miri (UB detection, nightly only)
miri:
    cargo +nightly miri test --package northroot-core
    cargo +nightly miri test --package northroot-canonical

# Fuzzing (requires cargo-fuzz)
fuzz target:
    cd crates/northroot-canonical && cargo fuzz run {{target}} -- -max_total_time=60

# Documentation
docs:
    cargo doc --workspace --no-deps

# Full nightly suite (slow)
nightly: fmt lint test golden docs coverage audit


#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"
export RUSTFLAGS="${RUSTFLAGS:--Dwarnings}"

before_status="$(mktemp)"
after_status="$(mktemp)"
trap 'rm -f "$before_status" "$after_status"' EXIT
git status --porcelain=v1 > "$before_status"

run() {
  printf '\n[northroot-verify] %s\n' "$*"
  "$@"
}

run cargo fmt --all --check
run cargo clippy --all-targets --all-features -- -D warnings
run cargo test --all --all-features
run cargo test --package northroot-canonical --test golden
run cargo test --workspace --doc
run cargo test --manifest-path apps/northroot/Cargo.toml
run python3 scripts/validate_schemas.py
run python3 scripts/validate_kernel_boundary.py
run python3 scripts/validate_nrj_fixtures.py
run python3 scripts/validate_state_recovery_invariants.py
run python3 scripts/validate_receipt_boundaries.py
run python3 scripts/v01_readiness.py --json
run python3 scripts/benchmark_journal_formats.py --scale 0.1 --quiet --out "${TMPDIR:-/tmp}/northroot-format-complexity-bench-smoke"

# Python promoted packages are outside the Cargo workspace.
run python3 -m compileall -q packages/northroot-durability/src packages/northroot-durability/tests
run env PYTHONPATH=packages/northroot-durability/src python3 -m unittest discover packages/northroot-durability/tests
run env PYTHONPATH=packages/northroot-durability/src python3 -m northroot.durability.cli public-check packages/northroot-durability/examples/public-artifacts.example.json

set +e
env PYTHONPATH=packages/northroot-durability/src python3 -m northroot.durability.cli public-check packages/northroot-durability/examples/private-artifacts.blocked.example.json >/tmp/northroot-private-artifact-check.out
private_check_status=$?
set -e
if [ "$private_check_status" -eq 0 ]; then
  cat /tmp/northroot-private-artifact-check.out >&2
  printf '\n[northroot-verify] private artifact fixture was unexpectedly allowed\n' >&2
  exit 1
fi
rm -f /tmp/northroot-private-artifact-check.out

find packages -type d -name __pycache__ -prune -exec rm -rf {} +

git status --porcelain=v1 > "$after_status"
if ! diff -u "$before_status" "$after_status"; then
  printf '\n[northroot-verify] working tree changed during verification\n' >&2
  exit 1
fi

printf '\n[northroot-verify] ok\n'

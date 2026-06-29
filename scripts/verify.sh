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
run python3 -c "import json; json.load(open('docs/security/agent-delegation-policy.dogfood.example.json', encoding='utf-8'))"
run python3 scripts/v01_readiness.py --json
run python3 scripts/benchmark_journal_formats.py --scale 0.1 --quiet --out "${TMPDIR:-/tmp}/northroot-format-complexity-bench-smoke"

# Python promoted packages are outside the Cargo workspace.
python_cache_prefix="${TMPDIR:-/tmp}/northroot-verify-pycache"
rm -rf "$python_cache_prefix"

run env PYTHONPYCACHEPREFIX="$python_cache_prefix" python3 -m compileall -q packages/northroot-durability/src packages/northroot-durability/tests
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-durability/src python3 -m unittest discover packages/northroot-durability/tests
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-durability/src python3 -m northroot.durability.cli public-check packages/northroot-durability/examples/public-artifacts.example.json

set +e
env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-durability/src python3 -m northroot.durability.cli public-check packages/northroot-durability/examples/private-artifacts.blocked.example.json >/tmp/northroot-private-artifact-check.out
private_check_status=$?
set -e
if [ "$private_check_status" -eq 0 ]; then
  cat /tmp/northroot-private-artifact-check.out >&2
  printf '\n[northroot-verify] private artifact fixture was unexpectedly allowed\n' >&2
  exit 1
fi
rm -f /tmp/northroot-private-artifact-check.out

run env PYTHONPYCACHEPREFIX="$python_cache_prefix" python3 -m compileall -q packages/northroot-custody/src packages/northroot-custody/tests
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m unittest discover packages/northroot-custody/tests
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m northroot.custody.cli validate packages/northroot-custody/examples/workspace-inventory.example.json --public-safe
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m northroot.custody.cli validate packages/northroot-custody/examples/custody-policy.example.json --public-safe
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m northroot.custody.cli validate packages/northroot-custody/examples/snapshot-plan.example.json --public-safe
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m northroot.custody.cli validate packages/northroot-custody/examples/verification-result.example.json --public-safe
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m northroot.custody.cli validate packages/northroot-custody/examples/retention-decision.example.json --public-safe
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m northroot.custody.cli validate packages/northroot-custody/examples/run-summary.example.json --public-safe
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m northroot.custody.cli validate packages/northroot-custody/examples/command-plan.example.json --public-safe
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m northroot.custody.cli validate packages/northroot-custody/examples/service-registry.example.json --public-safe
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m northroot.custody.cli validate packages/northroot-custody/examples/legacy-profile-import.redacted.example.json --public-safe
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m northroot.custody.cli validate packages/northroot-custody/examples/legacy-run-import.redacted.example.json --public-safe
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m northroot.custody.cli validate packages/northroot-custody/examples/secret-bindings.redacted.example.json --public-safe
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m northroot.custody.cli validate packages/northroot-custody/examples/secret-bindings.macos-keychain.example.json --public-safe
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m northroot.custody.cli validate packages/northroot-custody/examples/repository-bindings.redacted.example.json --public-safe
set +e
env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m northroot.custody.cli validate packages/northroot-custody/examples/private-bindings.blocked.example.json --public-safe >/tmp/northroot-custody-private-binding-check.out
custody_private_check_status=$?
set -e
if [ "$custody_private_check_status" -eq 0 ]; then
  cat /tmp/northroot-custody-private-binding-check.out >&2
  printf '\n[northroot-verify] private custody binding fixture was unexpectedly allowed\n' >&2
  exit 1
fi
rm -f /tmp/northroot-custody-private-binding-check.out
run env PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=packages/northroot-custody/src python3 -m northroot.custody.cli render-plan --inventory packages/northroot-custody/examples/workspace-inventory.example.json --policy packages/northroot-custody/examples/custody-policy.example.json

rm -rf "$python_cache_prefix"
find packages -type d -name __pycache__ -prune -exec rm -rf {} +

git status --porcelain=v1 > "$after_status"
if ! diff -u "$before_status" "$after_status"; then
  printf '\n[northroot-verify] working tree changed during verification\n' >&2
  exit 1
fi

printf '\n[northroot-verify] ok\n'

# Kernel Boundary

This document is the human-readable companion to the machine-readable
[`kernel-boundary.json`](../../kernel-boundary.json) contract.

Only `northroot-canonical` and `northroot-journal` are stable kernel crates.

The stable kernel owns deterministic canonical bytes, content-derived event
identity, `.nrj` append/read/verify behavior, and offline verification. It does
not own domain semantics, product policy authority, workflow orchestration,
agent runtimes, private deployments, SaaS adapters, or client operations.

`northroot-record`, `northroot-node`, and `northroot-state-eval` are substrate layers above the kernel.

`northroot-governance`, `northroot-execution`, `northroot-exchange`, and `northroot-ag` are capability or profile layers, not kernel crates.

`apps/northroot` is a CLI application, not a kernel crate.

Changing the kernel boundary requires updating `kernel-boundary.json` and passing `scripts/validate_kernel_boundary.py`.

## Protected Boundary Paths

The boundary contract treats these paths as protected because changes can alter
kernel identity rules, journal verification, or the enforcement surface itself:

- `Cargo.toml`
- `kernel-boundary.json`
- `scripts/validate_kernel_boundary.py`
- `docs/developer/kernel-boundary.md`
- `crates/northroot-canonical/**`
- `crates/northroot-journal/**`

## Dependency Rules

- `northroot-canonical` must not depend on any other `northroot-*` crate.
- `northroot-journal` may depend on `northroot-canonical` and no other `northroot-*` crate.
- Every Cargo workspace crate under `crates/` must be classified in `kernel-boundary.json`.
- `apps/northroot` must stay outside the Cargo workspace and be tested by manifest path.

## Verification

Run the validator directly:

```bash
python3 scripts/validate_kernel_boundary.py
```

The full repository gate runs it through:

```bash
bash scripts/verify.sh
```

# Quality & Testing Harness

The QA harness is runnable by humans, CI runners, containers, and agents. Codex
uses the same neutral scripts through compatibility wrappers.

## Quick Start

Fast checks before a PR:

```bash
just qa
```

Full handoff gate:

```bash
bash scripts/verify.sh
```

## Available Commands

- `just setup` - run `scripts/dev_setup.sh`
- `just verify` - run `scripts/verify.sh`
- `just fmt` - check formatting
- `just lint` - run clippy with warnings as errors
- `just test` - run Rust workspace tests
- `just golden` - run canonical golden tests
- `just schema` - validate JSON schemas
- `just cli-test` - run standalone CLI tests
- `just qa` - fast combined gate
- `just docs` - build Rust docs
- `just docs-test` - run doctests

Codex compatibility targets remain available: `just codex-setup` and
`just codex-verify`.

## Deep Checks

- `just coverage` - requires `cargo-llvm-cov`
- `just audit` - requires `cargo-deny` and `cargo-audit`
- `just miri` - requires nightly toolchain
- `just fuzz target <name>` - requires `cargo-fuzz`
- `just nightly` - slow local suite

## CI Workflows

Fast CI runs formatting, clippy, workspace tests, golden tests, schema
validation, docs, CLI tests, and repository verification. Nightly CI runs
coverage, audit, miri, fuzzing, and seeded redaction variations.

## Dependencies

Neutral setup:

```bash
bash scripts/dev_setup.sh
```

Optional deep-check tools:

```bash
NORTHROOT_DEEP_TOOLS=1 bash scripts/dev_setup.sh
```

Manual installs are still fine:

```bash
cargo install cargo-llvm-cov --locked
cargo install cargo-deny --locked
cargo install cargo-audit --locked
cargo install cargo-fuzz --locked
rustup toolchain install nightly
rustup component add miri --toolchain nightly
```

## Troubleshooting

- **Format fails**: run `cargo fmt --all`.
- **Clippy fails**: fix warnings; CI treats warnings as errors.
- **Golden tests fail**: confirm the canonicalization change is intentional before updating fixtures.
- **Private artifact check passes unexpectedly**: public/private durability policy regressed; inspect `northroot.durability.policy`.
- **Verification changes the tree**: remove generated caches or update the script to clean them before final status comparison.

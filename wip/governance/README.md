# Governance Event Schemas

This directory contains checkpoint and attestation event schemas that were moved out of the core trust kernel.

## Status

**Not part of core.** These are example schemas for governance events. The trust kernel operates on untyped `EventJson` and does not require these schemas.

## Contents

- `src/` - Rust types for checkpoint and attestation events
- `schemas/` - JSON Schema definitions
- `Cargo.toml` - Crate configuration (if used as a crate)

## Usage

If you need checkpoint/attestation events:
1. Copy or reference these schemas in your extension
2. Use `northroot-canonical` for canonicalization and event_id computation
3. Use `northroot-journal` for storage

## Future

These may be:
- Moved to a separate repository
- Published as a separate crate
- Kept as examples in `wip/`

The core trust kernel remains minimal and does not depend on these schemas.


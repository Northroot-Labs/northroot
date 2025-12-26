# Storage Abstraction Layer

This directory contains the storage abstraction layer that was moved out of the core trust kernel.

## Status

**Not part of core.** This is a convenience layer for typed event access and filtering. The trust kernel provides `northroot-journal` which is sufficient for core operations.

## Contents

- Storage traits (`StoreWriter`, `StoreReader`)
- Typed event parsing
- Event filtering
- Linkage navigation

## Usage

If you need storage abstractions:
1. Copy or reference this code in your extension
2. Build on top of `northroot-journal`
3. Add domain-specific typed events

## Future

This may be:
- Moved to a separate repository
- Published as a separate crate
- Kept as examples in `wip/`

The core trust kernel remains minimal and does not depend on this abstraction.

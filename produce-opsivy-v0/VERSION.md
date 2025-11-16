# Version Information

**Repository:** produce-opsivy-v0  
**Base Dependency:** northroot v0.1.0-alpha  
**Status:** Private repository for produce distributor operations

## Version History

### v0.1.0 (Initial)

- **Base:** northroot v0.1.0-alpha (frozen)
- **Schema:** Canonical SQLite schema for field operations
- **Events:** Three core events (load_intake, storage_placement, quality_sample)
- **Integration:** Northroot SDK for verifiable receipts

## Dependency Management

### Pinned Versions

- **northroot:** `v0.1.0-alpha` (frozen tag)

### Migration Strategy

When northroot releases v0.1.0 stable:

1. Test compatibility with v0.1.0 stable
2. Update dependency pin
3. Run full test suite
4. Update version to v0.2.0

## Versioning Policy

This repository follows independent versioning:

- **MAJOR**: Breaking changes to canonical data model or API
- **MINOR**: New events, features, or database schema additions
- **PATCH**: Bug fixes, documentation, examples

Changes to northroot core do not require version bumps in this repo (handled via dependency updates).


# Phase Documentation

This directory contains documentation for specific implementation phases from ADRs and plans.

## Phase Identification System

**Important**: Phases are **not global**. Each ADR or plan document has its own phase numbering system.

- **ADR-009-P4** (ADR-009 Phase 4) is different from **ADR-008-P2** (ADR-008 Phase 2)
- Each phase has a **canonical unique ID**: `ADR-XXX-PN` format
- Always reference phases using their canonical ID: `ADR-009-P4` not just "Phase 4"

### Foundation Phase Standard

**P01 is the standard foundation phase identifier** for highest priority foundational work that blocks all subsequent phases.

- **P01** = Foundation phase (highest priority, blocks all other phases)
- **P02, P03, P04...** = Subsequent phases in priority order
- This standard ensures consistent prioritization across all ADRs
- Schema allows `sequence: 1` (or higher) for foundation phases

See [ADR and Phase ID Standards](../process/ADR_PHASE_ID_STANDARDS.md) for complete identification system.

## Current Phases

### ADR-009: Hybrid ByteStream/RowMap Evidence Substrate

- **Phase 1**: ✅ Complete - Engine-internal `DataShape` enum + hash helper
- **Phase 2**: ✅ Complete - Extend `ExecutionPayload` to differentiate byte-level commitments
- **Phase 3**: ✅ Complete - Refactor Merkle Row-Map and ByteStream manifest builders
- **Phase 4** (`ADR-009-P4`): ✅ Complete - Privacy-Preserving Resolver API
  - Documentation: [ADR-009-P4.md](./ADR-009-P4.md)
- **Phase 5**: ⏭️ Pending - Summarized manifests for fast overlap
- **Phase 6**: ✅ Complete - Storage extensions
- **Phase 7**: ⏭️ Pending - Reuse reconciliation flow
- **Phase 8**: ⏭️ Pending - Helper functions for shape hash computation

See [ADR-009](../../ADRs/ADR-009.md) for full details.

## Phase Documentation Standards

### Required Header Format

All phase documentation must include YAML frontmatter with canonical phase ID:

```yaml
---
phase_id: ADR-XXX-PN
adr_id: ADR-XXX
adr_title: ADR Title
adr_path: ../../ADRs/ADR-XXX.md
phase_number: N
phase_title: Phase Description
status: complete | pending | in_progress | blocked
phase_type: additive | refactor | change
date_started: YYYY-MM-DD
date_completed: YYYY-MM-DD (optional)
---

# Phase N: Description (ADR-XXX)
```

See [ADR and Phase ID Standards](../process/ADR_PHASE_ID_STANDARDS.md) for complete metadata specification.

### Phase Classification

Per [Architectural Alignment Rules](../../.cursor/rules/architectural-alignment.mdc):

- **`refactor`**: Structural changes that preserve behavior
- **`change`**: Behavioral changes that modify outputs or contracts
- **`additive`**: New functionality that doesn't affect existing code

### Documentation Structure

1. **Header** (with ADR/plan reference)
2. **Classification** (phase type and rationale)
3. **Spec-First Tests** (if applicable)
4. **Implementation Checklist**
5. **Post-Implementation Verification**
6. **Related Documentation** (links to ADR, tests, etc.)

## Naming Conventions

- **Format**: `ADR-XXX-PN.md` (canonical) or `ADR-XXX-PN-description.md` (with optional description)
- **Examples**:
  - `ADR-009-P4.md` ✅ (canonical)
  - `ADR-009-P4-resolver-api.md` ✅ (optional descriptive suffix)
  - `ADR-008-P2.md` ✅
  - `ADR-001-P1.md` ✅

The phase ID in the filename must match the `phase_id` in the frontmatter.

## Archiving Completed Phases

After a phase is complete:

1. Update status to "✅ Complete" in the phase document
2. Update the parent ADR with completion status
3. Optionally move to `docs/phases/archive/` if no longer actively referenced
4. Keep for historical reference and audit trail

## Cross-References

When referencing phases in other documents:

- ✅ **Best**: Use canonical phase ID: `ADR-009-P4`
- ✅ **Good**: "ADR-009 Phase 4" or "Phase 4 of ADR-009"
- ❌ **Bad**: "Phase 4" (ambiguous)

Always use the canonical phase ID (`ADR-XXX-PN`) for unambiguous references.

### Examples

```markdown
See [ADR-009-P4](./ADR-009-P4.md) for resolver implementation.
The resolver API was implemented in ADR-009-P4.
See ADR-009-P4 for privacy-preserving artifact resolution.
```

See [ADR and Phase ID Standards](../process/ADR_PHASE_ID_STANDARDS.md) for complete cross-reference guidelines.


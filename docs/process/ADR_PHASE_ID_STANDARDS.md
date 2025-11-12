# ADR and Phase ID Standards

This document defines the canonical identification system for Architectural Decision Records (ADRs) and their implementation phases.

## ADR IDs

### Format
- **Canonical ID**: `ADR-XXX` where `XXX` is a zero-padded 3-digit number
- **Examples**: `ADR-001`, `ADR-009`, `ADR-042`

### File Naming
- **Preferred**: `ADR-XXX.md` (short form)
- **Legacy/Descriptive**: `ADR-XXX-description.md` (acceptable for existing files)
- **Examples**:
  - `ADR-009.md` ✅
  - `ADR-001-receipts-vs-engine.md` ✅ (legacy, acceptable)

### ADR Metadata
Each ADR should include in its header:
```markdown
# ADR-XXX: Title
Date: YYYY-MM-DD
Status: Proposed | Accepted | Superseded by ADR-YYY
```

## Phase IDs

### Format
- **Canonical ID**: `ADR-XXX-PN` where:
  - `ADR-XXX` = Parent ADR identifier
  - `P` = Phase prefix (literal)
  - `N` = Phase number within that ADR (no zero-padding)
- **Examples**:
  - `ADR-009-P4` = ADR-009, Phase 4
  - `ADR-008-P2` = ADR-008, Phase 2
  - `ADR-001-P1` = ADR-001, Phase 1

### File Naming
- **Preferred**: `ADR-XXX-PN.md` (canonical)
- **With Description**: `ADR-XXX-PN-description.md` (optional, for clarity)
- **Examples**:
  - `ADR-009-P4.md` ✅
  - `ADR-009-P4-resolver-api.md` ✅ (optional descriptive suffix)

### Phase Documentation Location
- **Active phases**: `docs/phases/ADR-XXX-PN.md`
- **Archived phases**: `docs/phases/archive/ADR-XXX-PN.md` (optional)

## Phase Documentation Metadata

All phase documentation must include YAML frontmatter for machine-readable linking:

```yaml
---
phase_id: ADR-009-P4
adr_id: ADR-009
adr_title: Hybrid ByteStream/RowMap Evidence Substrate
adr_path: ../../ADRs/ADR-009.md
phase_number: 4
phase_title: Privacy-Preserving Resolver API
status: complete | pending | in_progress | blocked
phase_type: additive | refactor | change
date_started: YYYY-MM-DD
date_completed: YYYY-MM-DD (optional)
---
```

### Metadata Fields

| Field | Required | Description |
|-------|----------|-------------|
| `phase_id` | ✅ | Canonical phase ID (e.g., `ADR-009-P4`) |
| `adr_id` | ✅ | Parent ADR ID (e.g., `ADR-009`) |
| `adr_title` | ✅ | Title of parent ADR |
| `adr_path` | ✅ | Relative path to ADR file |
| `phase_number` | ✅ | Phase number within ADR (integer) |
| `phase_title` | ✅ | Short descriptive title |
| `status` | ✅ | Current status |
| `phase_type` | ✅ | Classification per architectural alignment rules |
| `date_started` | ✅ | Start date (YYYY-MM-DD) |
| `date_completed` | ❌ | Completion date (if applicable) |

## Cross-Reference Standards

### In Documentation
- ✅ **Good**: `ADR-009-P4` or `[ADR-009-P4](docs/phases/ADR-009-P4.md)`
- ✅ **Good**: "See ADR-009-P4 for resolver implementation"
- ✅ **Good**: "Phase 4 of ADR-009 (ADR-009-P4)"
- ❌ **Bad**: "Phase 4" (ambiguous without ADR identifier)

### In Code Comments
```rust
// See ADR-009-P4 for resolver trait design
// Implements ArtifactResolver per ADR-009-P4
```

### In Commit Messages
```bash
feat(engine): implement ArtifactResolver trait (ADR-009-P4)
fix(storage): add encrypted locator storage (ADR-009-P4)
```

### In ADR Phase Tables
ADRs with phases should include a phase table:

```markdown
## Phases

| Phase ID | Description | Status | Documentation |
|----------|-------------|--------|---------------|
| ADR-009-P1 | Engine-internal DataShape enum | ✅ Complete | - |
| ADR-009-P2 | Extend ExecutionPayload | ✅ Complete | - |
| ADR-009-P4 | Privacy-Preserving Resolver API | ✅ Complete | [ADR-009-P4](../docs/phases/ADR-009-P4.md) |
| ADR-009-P5 | Summarized manifests | ⏭️ Pending | - |
```

## Optional Phase Documentation

Not all phases require separate documentation. Use these criteria:

### Document if:
- Complex implementation with multiple components
- Breaking changes or significant behavioral changes
- Requires spec-first tests or detailed planning
- Cross-cutting concerns affecting multiple crates
- Migration path needed

### Skip if:
- Simple, straightforward implementation
- Well-documented in the ADR itself
- Single component change
- No special considerations

When skipping documentation, list the phase in the ADR's phase table with status only (no documentation link).

## Phase Status Values

- **`pending`**: Not yet started
- **`in_progress`**: Currently being implemented
- **`complete`**: Implementation finished and verified
- **`blocked`**: Blocked by dependencies or external factors

## Phase Type Values

Per [Architectural Alignment Rules](../../.cursor/rules/architectural-alignment.mdc):

- **`refactor`**: Structural changes that preserve behavior
- **`change`**: Behavioral changes that modify outputs or contracts
- **`additive`**: New functionality that doesn't affect existing code

## Tooling and Automation

The frontmatter metadata enables:

1. **Phase Index Generation**: Automatically generate phase status pages
2. **Cross-Reference Validation**: Verify all phase references are valid
3. **Status Tracking**: Track phase completion across ADRs
4. **Documentation Linking**: Automatic bidirectional links between ADRs and phases

## Migration Notes

For existing phase documentation:
1. Add frontmatter metadata to existing phase docs
2. Rename files to canonical format (`ADR-XXX-PN.md`)
3. Update cross-references to use phase IDs
4. Update ADR phase tables with phase IDs and links

## Examples

### Complete Phase Documentation Example

```yaml
---
phase_id: ADR-009-P4
adr_id: ADR-009
adr_title: Hybrid ByteStream/RowMap Evidence Substrate
adr_path: ../../ADRs/ADR-009.md
phase_number: 4
phase_title: Privacy-Preserving Resolver API
status: complete
phase_type: additive
date_started: 2025-11-11
date_completed: 2025-11-12
---

# Phase 4: Privacy-Preserving Resolver API (ADR-009)

[Phase documentation content...]
```

### ADR Phase Table Example

```markdown
## Implementation Phases

| Phase ID | Description | Status | Documentation |
|----------|-------------|--------|---------------|
| ADR-009-P1 | Engine-internal `DataShape` enum + hash helper | ✅ Complete | - |
| ADR-009-P2 | Extend `ExecutionPayload` to differentiate byte-level commitments | ✅ Complete | - |
| ADR-009-P3 | Refactor Merkle Row-Map and ByteStream manifest builders | ✅ Complete | - |
| ADR-009-P4 | Privacy-Preserving Resolver API | ✅ Complete | [ADR-009-P4](../docs/phases/ADR-009-P4.md) |
| ADR-009-P5 | Summarized manifests for fast overlap | ⏭️ Pending | - |
| ADR-009-P6 | Storage extensions | ✅ Complete | - |
| ADR-009-P7 | Reuse reconciliation flow | ⏭️ Pending | - |
| ADR-009-P8 | Helper functions for shape hash computation | ⏭️ Pending | - |
```


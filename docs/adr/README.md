# Architectural Decision Records (ADRs)

This directory contains all Architectural Decision Records (ADRs) in a machine-readable, agent-friendly format.

## Structure

Each ADR has its own directory:

```
docs/adr/
  ADR-0001-receipts-vs-engine-boundaries/
    ADR-0001.md              # Main ADR document (Markdown + YAML frontmatter)
    phases/                  # Phase entries (YAML files)
      ADR-0001-P01.yaml
    attachments/             # Supporting files (diagrams, PoCs, benchmarks)
  ADR-0002-.../
  ...
  adr.index.json            # Machine-readable index
```

## ID Scheme

- **ADR ID**: `ADR-####` (e.g., `ADR-0001`, `ADR-0009`)
- **Phase ID**: `ADR-####-P##` (e.g., `ADR-0009-P04`)
- **Directory**: `ADR-####-slug` (e.g., `ADR-0001-receipts-vs-engine-boundaries`)

## Lifecycle

Phases follow this lifecycle:

```
draft → proposed → (accepted | rejected) → (implemented | superseded | deprecated)
```

The ADR's `status` field is derived from the latest accepted phase.

## Machine-Readable Format

### ADR Frontmatter

Each ADR markdown file includes YAML frontmatter:

```yaml
---
$schema: ../../schemas/adr/adr.schema.json
adr_id: ADR-0001
slug: receipts-vs-engine-boundaries
title: Receipts vs Engine Boundaries
created_at: 2025-11-08T00:00:00Z
status: accepted
domain: engine
tags: [receipts, engine, boundaries]
related:
  supersedes: []
  superseded_by: []
  depends_on: []
  related_to: []
---
```

### Phase Files

Phase entries are separate YAML files in the `phases/` subdirectory:

```yaml
---
$schema: ../../../schemas/adr/phase.schema.json
phase_id: ADR-0009-P04
adr_id: ADR-0009
sequence: 4
title: Privacy-Preserving Resolver API
status: implemented
summary: Created resolver.rs module with ArtifactResolver and ManagedCache traits.
proposed_at: 2025-11-11T00:00:00Z
decided_at: 2025-11-11T00:00:00Z
---
```

## Index

The `adr.index.json` file provides a machine-readable index of all ADRs and their phases. It's automatically generated and should not be edited manually.

## Tools

### Generate Index

```bash
make adr.index
```

### Validate Structure

```bash
make adr.validate
```

### Migrate Existing ADRs

```bash
make adr.migrate
```

### Watch for Changes

```bash
make adr.watch  # Requires watchexec: brew install watchexec
```

## Schemas

JSON schemas for validation are in `schemas/adr/`:

- `adr.schema.json` - ADR frontmatter schema
- `phase.schema.json` - Phase file schema
- `adr.index.schema.json` - Index file schema

## References

- [ADR and Phase ID Standards](../process/ADR_PHASE_ID_STANDARDS.md)
- [ADR Playbook](../process/ADR_PLAYBOOK.md)


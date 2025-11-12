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

### Index Version 0.4

The index (version 0.4) includes rich metadata:

- **ADR-level fields**: `accepted_at`, `implemented_at`, `phase_ids`, `depends_on`, `code_refs`
- **Phase-level fields**: `decision_drivers`, `tasks`, `files`, `success_criteria`
- **Code references**: Links to files and symbols touched by implementation commits

## Graph Structure

The ADR system generates graph files for efficient traversal and analysis:

- **`graph.nodes.jsonl`**: JSONL file with node entries (ADR, PHASE, COMMIT, FILE, SYMBOL)
- **`graph.edges.jsonl`**: JSONL file with edge entries (HAS_PHASE, IMPLEMENTS_IN, TOUCHES_FILE, TOUCHES_SYMBOL, DEPENDS_ON)

### Node Types

- **ADR**: `{"id":"adr:0001","type":"ADR","adr_id":"ADR-0001",...}`
- **PHASE**: `{"id":"phase:0001:P03","type":"PHASE","phase_id":"ADR-0001-P03-...",...}`
- **COMMIT**: `{"id":"commit:4d5e6f","type":"COMMIT","sha":"4d5e6f...",...}`
- **FILE**: `{"id":"file:crates/config/src/validator.rs","type":"FILE","path":"..."}`
- **SYMBOL**: `{"id":"sym:ConfigValidator::validate","type":"SYMBOL","path":"...","symbol":"..."}`

### Edge Types

- **HAS_PHASE**: `{"src":"adr:0001","dst":"phase:0001:P01","type":"HAS_PHASE"}`
- **IMPLEMENTS_IN**: `{"src":"phase:0001:P03","dst":"commit:4d5e6f","type":"IMPLEMENTS_IN"}`
- **TOUCHES_FILE**: `{"src":"commit:4d5e6f","dst":"file:...","type":"TOUCHES_FILE"}`
- **TOUCHES_SYMBOL**: `{"src":"commit:4d5e6f","dst":"sym:...","type":"TOUCHES_SYMBOL"}`
- **DEPENDS_ON**: `{"src":"adr:0001","dst":"adr:0003","type":"DEPENDS_ON"}`

The JSONL format enables streaming and chunking for large graphs.

## Tools

### Generate Index

```bash
make adr.index
```

Generates the ADR index (`adr.index.json`) with all metadata from ADR and phase files.

### Generate Graph

```bash
make adr.graph
```

Generates graph files (`graph.nodes.jsonl`, `graph.edges.jsonl`) from the index.

### Link Commits to Code

```bash
make adr.link
```

Links implementation commits to files and symbols, updating graph files and populating `code_refs` in the index. Uses `git` to map commits to files, and optionally `ctags` for symbol extraction.

### Full Pipeline

```bash
make adr.all
```

Runs the complete pipeline: `adr.index` → `adr.graph` → `adr.link`

### Validate Structure

```bash
make adr.validate
```

Validates ADR structure, schemas, commit SHA provenance, and graph consistency.

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
- `adr.index.schema.json` - Index file schema (version 0.4)
- `graph.nodes.schema.json` - Graph node schema
- `graph.edges.schema.json` - Graph edge schema

## References

- [ADR and Phase ID Standards](../process/ADR_PHASE_ID_STANDARDS.md)
- [ADR Playbook](../process/ADR_PLAYBOOK.md)


# Documentation Quick Index

**Quick reference for finding documentation by topic.**

For the complete table of contents, see [README.md](README.md).

## By Topic

### Getting Started
- [Main README](../README.md) - Project overview
- [Installation Guide](guides/installation.md) - Setup
- [Hello Receipts Demo](../sdk/python/northroot/examples/hello_receipts.py) - 3-step demo

### Core Concepts
- [Proof Algebra](specs/proof_algebra.md) ⭐ **START HERE**
- [Delta Compute](specs/delta_compute.md) - Reuse decisions
- [Incremental Compute](specs/incremental_compute.md) - Strategy
- [Merkle Row-Map](specs/merkle_row_map.md) - State structure

### Python SDK
- [Python SDK README](../sdk/python/northroot/README.md) - Full API
- [Examples](../sdk/python/northroot/examples/) - Code samples
- [Canonical Forms](guides/canonical-forms-reference.md) - CBOR/JCS

### Architecture
- [Architecture Diagrams](specs/architecture-diagrams.md) - Visual design
- [ADR-0012: Goal Grid Alignment](adr/ADR-0012-harada-v01-alignment/ADR-0012.md) ⭐ **ACTIVE**
- [Engine Analysis](analysis/engine-0.1.0-analysis.md) - v0.1 analysis

### Development
- [Versioning](process/VERSIONING.md) - Version strategy
- [Release Workflow](process/RELEASE_WORKFLOW.md) - Release process
- [Commit Guide](process/COMMIT_GUIDE.md) - Commit conventions
- [ADR Playbook](process/ADR_PLAYBOOK.md) - ADR process

## By File Type

### Specifications (`specs/`)
- `proof_algebra.md` - Core algebra
- `delta_compute.md` - Reuse decisions
- `incremental_compute.md` - Strategy
- `merkle_row_map.md` - State structure
- `hashing-and-domain-separation.md` - Crypto rules
- `v01-chunk-model-freeze.md` - Data model
- `architecture-diagrams.md` - Visual design

### Guides (`guides/`)
- `installation.md` - Setup
- `canonical-forms-reference.md` - CBOR/JCS

### Process (`process/`)
- `VERSIONING.md` - Versioning
- `RELEASE_WORKFLOW.md` - Releases
- `COMMIT_GUIDE.md` - Commits
- `CHANGELOG_GUIDE.md` - Changelog
- `ADR_PLAYBOOK.md` - ADRs
- `ARCHITECTURAL_ALIGNMENT_PLAYBOOK.md` - Alignment
- `INTEGRITY_CHECKS.md` - Testing
- `ADR_PHASE_ID_STANDARDS.md` - Naming

### ADRs (`adr/`)
- `README.md` - ADR overview
- `ADR-0012-*/ADR-0012.md` ⭐ **ACTIVE**
- `ADR-0001-*/ADR-0001.md` through `ADR-0011-*/ADR-0011.md` - Historical

### Analysis & Planning
- `analysis/engine-0.1.0-analysis.md` - Analysis
- `planning/v01-execution-plan.md` - Execution plan
- `planning/v01-dependency-graph.md` - Dependencies
- `planning/goal-grid.md` - Goal grid framework

### API & CI
- `api/PUBLIC_API_INVENTORY.md` - APIs
- `ci/CI_VECTOR_CHECKS.md` - CI checks

## By Role

### New User
1. [Main README](../README.md)
2. [Installation Guide](guides/installation.md)
3. [Hello Receipts](../sdk/python/northroot/examples/hello_receipts.py)

### Developer
1. [Proof Algebra](specs/proof_algebra.md)
2. [Python SDK README](../sdk/python/northroot/README.md)
3. [Public API Inventory](api/PUBLIC_API_INVENTORY.md)

### Architect
1. [ADR-0012: Goal Grid](adr/ADR-0012-harada-v01-alignment/ADR-0012.md)
2. [Architecture Diagrams](specs/architecture-diagrams.md)
3. [Delta Compute Spec](specs/delta_compute.md)

### Maintainer
1. [Release Workflow](process/RELEASE_WORKFLOW.md)
2. [Versioning](process/VERSIONING.md)
3. [Commit Guide](process/COMMIT_GUIDE.md)

---

See [README.md](README.md) for the complete table of contents.


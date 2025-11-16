# Northroot Documentation

**Complete guide to understanding, using, and contributing to Northroot.**

This is your starting point for all documentation. Use the table of contents below to navigate to specific topics.

## Table of Contents

### 🚀 Getting Started

**For new users who want to get up and running quickly:**

- **[Main README](../README.md)** - Project overview, problem statement, and quick start
- **[Installation Guide](guides/installation.md)** - Setup instructions for all platforms
- **[Hello Receipts Demo](../sdk/python/northroot/examples/hello_receipts.py)** - Simplest 3-step demo
- **[Python SDK README](../sdk/python/northroot/README.md)** - Full Python SDK documentation

### 📖 User Guides

**Step-by-step guides for using Northroot:**

- **[Installation Guide](guides/installation.md)** - Installation and setup
- **[Canonical Forms Reference](guides/canonical-forms-reference.md)** - CBOR, JCS, and JSON adapters
- **[Python SDK Examples](../sdk/python/northroot/examples/)** - Working code examples

### 📐 Specifications

**Technical specifications and formal definitions:**

- **[Proof Algebra](specs/proof_algebra.md)** ⭐ **START HERE** - Unified algebra overview
- **[Delta Compute](specs/delta_compute.md)** - Formal reuse decision specification
- **[Incremental Compute](specs/incremental_compute.md)** - Incremental recomputation strategy
- **[Merkle Row-Map](specs/merkle_row_map.md)** - Deterministic state structure
- **[Hashing and Domain Separation](specs/hashing-and-domain-separation.md)** - Cryptographic hashing rules
- **[v0.1 Chunk Model Freeze](specs/v01-chunk-model-freeze.md)** - Frozen data model for v0.1
- **[Architecture Diagrams](specs/architecture-diagrams.md)** - Visual system architecture

### 🏗️ Architecture & Design

**Architectural decisions, analysis, and design documents:**

#### Architectural Decision Records (ADRs)

- **[ADR Index](adr/README.md)** - Overview of ADR system
- **[ADR-0012: Goal Grid v0.1 Alignment](adr/ADR-0012-harada-v01-alignment/ADR-0012.md)** ⭐ **ACTIVE** - Current roadmap
- **[Goal Grid Progress Tracking](adr/ADR-0012-harada-v01-alignment/HARADA_PROGRESS.md)** - Task completion status

**Historical ADRs (Superseded by ADR-0012):**
- [ADR-0001: Receipts vs Engine Boundaries](adr/ADR-0001-receipts-vs-engine-boundaries/ADR-0001.md)
- [ADR-0002: Canonicalization Strategy (CBOR)](adr/ADR-0002-canonicalization-strategy-cbor/ADR-0002.md)
- [ADR-0003: Delta Compute Decisions](adr/ADR-0003-delta-compute-decisions-recorded-under-spendjustif/ADR-0003.md)
- [ADR-0004: Identity Root Commitment](adr/ADR-0004-identity-root-commitment/ADR-0004.md)
- [ADR-0005: Engine Architecture](adr/ADR-0005-engine-architecture/ADR-0005.md)
- [ADR-0006: Signature Verification Strategy](adr/ADR-0006-signature-verification-strategy/ADR-0006.md)
- [ADR-0007: Delta Compute Implementation](adr/ADR-0007-delta-compute-implementation-strategy/ADR-0007.md)
- [ADR-0008: Proof-Addressable Storage](adr/ADR-0008-proof-addressable-storage-and-incremental-compute-/ADR-0008.md)
- [ADR-0009: Hybrid ByteStream/RowMap Evidence](adr/ADR-0009-hybrid-bytestreamrowmap-evidence-substrate-with-pr/ADR-0009.md)
- [ADR-0010: v0.1 Release Readiness](adr/ADR-0010-v010-release-readiness-evaluation/ADR-0010.md)
- [ADR-0011: Manifest Storage Architecture](adr/ADR-0011-manifest-storage-and-retrieval-architecture/ADR-0011.md)

**ADR Support Documents:**
- [ADR Playbook](process/ADR_PLAYBOOK.md) - How to create and manage ADRs
- [ADR Phase ID Standards](process/ADR_PHASE_ID_STANDARDS.md) - Naming conventions
- [Commit SHA Provenance](adr/COMMIT_SHA_PROVENANCE.md) - ADR implementation tracking
- [ADR Enforcement Summary](adr/ENFORCEMENT_SUMMARY.md) - Validation rules
- [ADR Migration Summary](adr/MIGRATION_SUMMARY.md) - Migration history
- [ADR Research Summary](adr/RESEARCH_SUMMARY.md) - Research context

#### Analysis & Planning

- **[Engine v0.1.0 Analysis](analysis/engine-0.1.0-analysis.md)** - Critical analysis for v0.1.0
- **[v0.1 Execution Plan](planning/v01-execution-plan.md)** - Detailed execution plan
- **[v0.1 Dependency Graph](planning/v01-dependency-graph.md)** - Task dependencies
- **[Goal Grid](planning/goal-grid.md)** - Planning framework used for v0.1

### 🔧 Development Process

**For contributors and maintainers:**

#### Versioning & Releases

- **[Versioning Strategy](process/VERSIONING.md)** - Version numbering and coordination
- **[Release Workflow](process/RELEASE_WORKFLOW.md)** - Step-by-step release process
- **[Changelog Guide](process/CHANGELOG_GUIDE.md)** - Automatic changelog generation

#### Code Quality & Standards

- **[Commit Guide](process/COMMIT_GUIDE.md)** - Commit message conventions
- **[Architectural Alignment Playbook](process/ARCHITECTURAL_ALIGNMENT_PLAYBOOK.md)** - Phase classification
- **[Integrity Checks](process/INTEGRITY_CHECKS.md)** - Test vector validation

#### ADR Process

- **[ADR Playbook](process/ADR_PLAYBOOK.md)** - ADR creation and management
- **[ADR Phase ID Standards](process/ADR_PHASE_ID_STANDARDS.md)** - Naming conventions
- **[Phase Documentation](phases/README.md)** - Phase documentation standards

### 📚 API Reference

**API documentation and public interfaces:**

- **[Public API Inventory](api/PUBLIC_API_INVENTORY.md)** - All public APIs across crates
- **[Python SDK README](../sdk/python/northroot/README.md)** - Python SDK API reference

### 🔍 CI/CD & Testing

**Continuous integration and testing documentation:**

- **[CI Vector Checks](ci/CI_VECTOR_CHECKS.md)** - Test vector integrity in CI

### 📁 Documentation Structure

```
docs/
├── README.md                    # This file - documentation index
├── guides/                      # User-facing how-to guides
│   ├── installation.md
│   └── canonical-forms-reference.md
├── specs/                       # Technical specifications
│   ├── proof_algebra.md        # ⭐ Start here for understanding
│   ├── delta_compute.md
│   ├── incremental_compute.md
│   ├── merkle_row_map.md
│   ├── hashing-and-domain-separation.md
│   ├── v01-chunk-model-freeze.md
│   └── architecture-diagrams.md
├── adr/                         # Architectural Decision Records
│   ├── README.md               # ADR overview
│   ├── ADR-0012-.../          # ⭐ Active ADR (goal grid alignment)
│   └── ADR-0001-.../          # Historical ADRs
├── process/                     # Development process guides
│   ├── VERSIONING.md
│   ├── RELEASE_WORKFLOW.md
│   ├── COMMIT_GUIDE.md
│   ├── CHANGELOG_GUIDE.md
│   ├── ADR_PLAYBOOK.md
│   └── ...
├── analysis/                    # Technical analysis
│   └── engine-0.1.0-analysis.md
├── planning/                    # Planning documents
│   ├── v01-execution-plan.md
│   └── v01-dependency-graph.md
├── api/                         # API documentation
│   └── PUBLIC_API_INVENTORY.md
├── ci/                          # CI/CD documentation
│   └── CI_VECTOR_CHECKS.md
└── phases/                       # Phase-specific docs
    └── README.md
```

## Quick Navigation by Role

### 👤 New User

1. Read [Main README](../README.md) for overview
2. Follow [Installation Guide](guides/installation.md)
3. Try [Hello Receipts Demo](../sdk/python/northroot/examples/hello_receipts.py)
4. Explore [Python SDK Examples](../sdk/python/northroot/examples/)

### 👨‍💻 Developer

1. Read [Proof Algebra](specs/proof_algebra.md) for core concepts
2. Review [Public API Inventory](api/PUBLIC_API_INVENTORY.md)
3. Check [Python SDK README](../sdk/python/northroot/README.md)
4. See [Canonical Forms Reference](guides/canonical-forms-reference.md)

### 🏗️ Architect / Contributor

1. Read [ADR-0012: Goal Grid Alignment](adr/ADR-0012-harada-v01-alignment/ADR-0012.md)
2. Review [Architecture Diagrams](specs/architecture-diagrams.md)
3. Study [Delta Compute Spec](specs/delta_compute.md)
4. Check [Engine Analysis](analysis/engine-0.1.0-analysis.md)

### 🔧 Maintainer

1. Follow [Release Workflow](process/RELEASE_WORKFLOW.md)
2. Use [Versioning Strategy](process/VERSIONING.md)
3. Read [Commit Guide](process/COMMIT_GUIDE.md)
4. Review [ADR Playbook](process/ADR_PLAYBOOK.md)

## Documentation Standards

- **Specifications** are versioned and maintained
- **ADRs** follow the playbook format and are machine-readable
- **Guides** are user-facing and kept up-to-date
- **Process docs** are for contributors and maintainers
- **Analysis docs** are kept for historical reference

## Finding What You Need

### By Topic

- **Installation & Setup**: [Installation Guide](guides/installation.md)
- **Core Concepts**: [Proof Algebra](specs/proof_algebra.md)
- **Delta Compute**: [Delta Compute Spec](specs/delta_compute.md)
- **Python SDK**: [Python SDK README](../sdk/python/northroot/README.md)
- **Architecture**: [Architecture Diagrams](specs/architecture-diagrams.md)
- **Releases**: [Release Workflow](process/RELEASE_WORKFLOW.md)
- **Contributing**: [CONTRIBUTING.md](../CONTRIBUTING.md)

### By File Type

- **Specifications**: `docs/specs/*.md`
- **Guides**: `docs/guides/*.md`
- **ADRs**: `docs/adr/ADR-XXXX-*/ADR-XXXX.md`
- **Process**: `docs/process/*.md`
- **Analysis**: `docs/analysis/*.md`
- **Planning**: `docs/planning/*.md`

## Related Resources

- **[Main README](../README.md)** - Project overview and quick start
- **[CONTRIBUTING.md](../CONTRIBUTING.md)** - Contribution guidelines
- **[RELEASING.md](../RELEASING.md)** - Release process (legacy, see [Release Workflow](process/RELEASE_WORKFLOW.md))
- **[CHANGELOG.md](../CHANGELOG.md)** - Project changelog
- **[Python SDK README](../sdk/python/northroot/README.md)** - Python SDK documentation
- **[Research Reports](../research/reports/)** - Research and analysis reports

## Questions?

- **Usage questions**: Check [Python SDK README](../sdk/python/northroot/README.md) or open an issue
- **Architecture questions**: See [ADRs](adr/README.md) or [Architecture Diagrams](specs/architecture-diagrams.md)
- **Contributing**: Read [CONTRIBUTING.md](../CONTRIBUTING.md) and [ADR Playbook](process/ADR_PLAYBOOK.md)

---

**Last Updated:** 2025-11-16  
**Documentation Version:** v0.1.0

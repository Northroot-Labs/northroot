# Documentation Index

This directory contains all project documentation organized by category.

## Structure

### `/process/` - Development Process Guides
Process documentation for development workflows and standards:
- **ADR_PLAYBOOK.md** - Architectural Decision Record (ADR) process and guidelines
- **ARCHITECTURAL_ALIGNMENT_PLAYBOOK.md** - Phase classification and alignment verification process
- **CHANGELOG_GUIDE.md** - Changelog generation and management guide
- **INTEGRITY_CHECKS.md** - Test vector and baseline integrity verification process

### `/ci/` - CI/CD Documentation
Continuous Integration and Deployment documentation:
- **CI_VECTOR_CHECKS.md** - CI checks for test vector integrity

### `/phases/` - Phase-Specific Documentation
Documentation for specific implementation phases from ADRs and plans:
- **README.md** - Phase documentation standards and identification system
- **ADR-009-P4.md** - ADR-009 Phase 4 (Privacy-Preserving Resolver API) - ✅ Complete

> **Note**: Phases are ADR/plan-specific, not global. Each phase has a canonical unique ID: `ADR-XXX-PN`. Always reference phases using their canonical ID (e.g., `ADR-009-P4`). See [ADR and Phase ID Standards](../docs/process/ADR_PHASE_ID_STANDARDS.md).

### `/analysis/` - Analysis Documents
Technical analysis and assessment documents:
- **engine-0.1.0-analysis.md** - Critical analysis and prioritization for engine v0.1.0

### `/planning/` - Planning Documents
Planning and design documents:
- **production-compute-jobs.md** - Production compute job examples and planning

### `/specs/` - Specifications
Technical specifications and architecture documentation:
- **proof_algebra.md** - Proof algebra specification
- **delta_compute.md** - Delta compute specification
- **incremental_compute.md** - Incremental compute specification
- **merkle_row_map.md** - Merkle Row-Map specification
- **architecture-diagrams.md** - Architecture diagrams

## Quick Links

- **Getting Started**: See [README.md](../README.md) in project root
- **Contributing**: See [CONTRIBUTING.md](../CONTRIBUTING.md)
- **Releasing**: See [RELEASING.md](../RELEASING.md)
- **Architectural Decisions**: See [ADRs/](../ADRs/) directory
- **Research Reports**: See [research/reports/](../research/reports/)

## Documentation Standards

- All process guides follow the playbook format
- Phase-specific docs are archived after phase completion
- Analysis documents are kept for historical reference
- Specifications are versioned and maintained


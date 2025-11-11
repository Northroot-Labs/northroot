# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Automatic changelog generation toolchain

### Changed
- Migrated from JCS to CBOR canonicalization (RFC 8949) for receipts
- Updated Merkle Row-Map to use CBOR canonicalization with string-based domain separation

### Fixed
- Integrity check script now excludes README.md files from vector checks

## [0.1.0] - 2025-01-XX

### Added
- Unified receipt algebra system with six receipt kinds (data_shape, method_shape, reasoning_shape, execution, spend, settlement)
- CBOR deterministic encoding (RFC 8949) for canonicalization
- JSON adapter layer for external compatibility
- CBOR Diagnostic Notation (CDN) for human-readable debugging
- Proof-addressable cache (PAC) key computation
- Delta compute strategies (partition, incremental sum)
- Merkle Row-Map for deterministic state representation
- SQLite storage adapter for receipt persistence
- Three demo examples (FinOps cost attribution, ETL partition reuse, analytics dashboard)
- Integrity check system for test vectors and baselines
- Comprehensive test suite with drift detection

### Changed
- Receipt canonicalization from JSON (JCS) to CBOR (RFC 8949)
- Merkle tree domain separation from byte prefixes to string prefixes

### Security
- Input validation at receipt boundaries
- Deterministic encoding prevents canonicalization attacks

---

## Changelog Format

Entries are categorized as:
- **Added**: New features
- **Changed**: Changes in existing functionality
- **Deprecated**: Soon-to-be removed features
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Security-related changes


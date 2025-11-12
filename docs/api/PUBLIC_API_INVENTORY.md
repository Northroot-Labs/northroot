# Public API Inventory

This document provides a comprehensive inventory of all public APIs across the Northroot crates.

**Last Updated**: 2025-01-12  
**Status**: In Progress (Phase 1 - Foundation)

## Purpose

This inventory serves to:
- Document the public API surface for SDK development
- Identify API stability concerns
- Guide versioning strategy
- Support API design reviews

## Crate: `northroot-engine`

### Core Modules

#### `delta` Module
**Purpose**: Delta compute operations (reuse decisions, overlap computation)

**Public Functions**:
- `decide_reuse(overlap_j: f64, cost_model: &CostModel, row_count: Option<usize>) -> (ReuseDecision, ReuseJustification)`
  - **Stability**: Stable (v0.1.0)
  - **Breaking Changes**: None expected
  
- `economic_delta(overlap_j: f64, cost_model: &CostModel, row_count: Option<usize>) -> f64`
  - **Stability**: Stable (v0.1.0)
  - **Breaking Changes**: None expected

- `jaccard_similarity(set1: &HashSet<String>, set2: &HashSet<String>) -> f64`
  - **Stability**: Stable (v0.1.0)
  - **Breaking Changes**: None expected

**Public Types**:
- `ChunkSet` - Set of chunk IDs for overlap computation
- `ReuseDecision` - Enum: Reuse/Recompute
- `ReuseJustification` - Detailed justification for reuse decision

#### `shapes` Module
**Purpose**: Data shape representation and hash computation

**Public Functions**:
- `compute_data_shape_hash(shape: &DataShape) -> Result<String, DataShapeError>`
  - **Stability**: Stable (v0.1.0)
  - **Breaking Changes**: None expected

**Public Types**:
- `DataShape` - Enum: ByteStream/RowMap
- `ChunkScheme` - Enum: CDC/Fixed
- `DataShapeError` - Error type for shape operations

#### `cas` Module
**Purpose**: Content-addressable storage operations

**Public Functions**:
- `chunk_by_cdc(data: &[u8], avg_size: u64) -> Result<Vec<Chunk>, CasError>`
  - **Stability**: Stable (v0.1.0)
  - **Breaking Changes**: None expected

- `chunk_by_fixed(data: &[u8], size: u64) -> Result<Vec<Chunk>, CasError>`
  - **Stability**: Stable (v0.1.0)
  - **Breaking Changes**: None expected

- `build_bytestream_manifest(chunks: &[Chunk], scheme: ChunkScheme) -> Result<ByteStreamManifest, CasError>`
  - **Stability**: Stable (v0.1.0)
  - **Breaking Changes**: None expected

**Public Types**:
- `Chunk` - Chunk structure (id, offset, len, hash)
- `ByteStreamManifest` - Manifest structure
- `CasError` - Error type for CAS operations

#### `composition` Module
**Purpose**: Receipt composition operations

**Public Functions**:
- `build_sequential_chain(receipts: &[Receipt]) -> Result<Receipt, CompositionError>`
  - **Stability**: Stable (v0.1.0)
  - **Breaking Changes**: None expected

- `validate_sequential(receipts: &[Receipt]) -> Result<(), CompositionError>`
  - **Stability**: Stable (v0.1.0)
  - **Breaking Changes**: None expected

**Public Types**:
- `CompositionError` - Error type for composition operations

#### `execution` Module
**Purpose**: Execution tracking and Merkle row-map

**Public Functions**:
- `generate_trace_id() -> Uuid`
  - **Stability**: Stable (v0.1.0)
  - **Breaking Changes**: None expected

**Public Types**:
- `MerkleRowMap` - Merkle tree over row map
- `ExecutionReceiptBuilder` - Builder for execution receipts

### Re-exported from `northroot-policy`

**Note**: Policy validation lives in `northroot-policy` crate. Engine re-exports for convenience.

- `load_policy(policy_json: &str) -> Result<DeltaComputePolicy, PolicyError>`
- `validate_policy(policy: &DeltaComputePolicy) -> Result<(), PolicyError>`
- `CostModel` - Cost model structure

## Crate: `northroot-receipts`

### Core Types

- `Receipt` - Main receipt structure
  - **Stability**: Stable (v0.1.0)
  - **Breaking Changes**: None expected for v0.1.0

- `Payload` - Enum of payload types (DataShape, MethodShape, Execution, etc.)
  - **Stability**: Stable (v0.1.0)
  - **Breaking Changes**: New variants may be added (non-breaking)

- `ValidationError` - Error type for receipt validation
  - **Stability**: Stable (v0.1.0)

### Public Functions

- `Receipt::validate(&self) -> Result<(), ValidationError>`
  - **Stability**: Stable (v0.1.0)

## Crate: `northroot-storage`

### Traits

- `ReceiptStore` - Trait for receipt storage backends
  - **Stability**: Stable (v0.1.0)
  - **Breaking Changes**: None expected

- `ManifestStore` - Trait for manifest storage
  - **Stability**: Stable (v0.1.0)

### Implementations

- `SqliteStore` - SQLite implementation
  - **Stability**: Stable (v0.1.0)

## API Stability Policy

### v0.1.0 (Current)

- **Status**: Pre-release
- **Breaking Changes**: Allowed (with migration notes)
- **Additions**: Allowed
- **Removals**: Allowed (with deprecation notice)

### v0.2.0+ (Post-release)

- **Status**: Stable
- **Breaking Changes**: Require major version bump
- **Additions**: Allowed (minor version bump)
- **Removals**: Require deprecation period (minor version) then major version bump

## SDK Exposure Strategy

### Phase 1 (v0.1.0) - Core APIs

**Priority APIs for Python SDK**:
1. `decide_reuse()` - Reuse decision logic
2. `jaccard_similarity()` - Overlap computation
3. `compute_data_shape_hash()` - Shape hash computation
4. `Receipt` creation and validation
5. `chunk_by_cdc()` / `chunk_by_fixed()` - Chunking operations

### Phase 2 (v0.2.0+) - Extended APIs

- Storage operations
- Advanced composition operations
- Policy validation APIs

## Notes

- All APIs are documented with rustdoc
- Error types use `thiserror` for better error messages
- APIs follow consistent naming conventions
- Return types use `Result<T, E>` for error handling


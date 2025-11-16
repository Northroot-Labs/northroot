//! Northroot Engine: Proof algebra implementation and execution engine.
//!
//! This crate provides the core engine for the Northroot proof algebra system,
//! including receipt composition, validation, and execution tracking.
//!
//! ## Architecture Boundaries
//!
//! **What this crate does**: Computation, composition, execution tracking, delta decisions
//! - Root computations (set, sequence, tensor, Merkle)
//! - Receipt composition (sequential, parallel)
//! - Execution state management
//! - Delta compute (chunking, reuse decisions)
//! - Signature verification
//!
//! **What this crate does NOT do**:
//! - Policy validation (see `northroot-policy`) - answers "is this allowed?"
//! - Receipt structure validation (see `northroot-receipts`) - answers "is this well-formed?"
//! - Operator/manifest definitions (see `northroot-ops`) - answers "what can be run?"
//!
//! **Dependencies**:
//! - Depends on: `commons`, `receipts`, `policy`
//! - Must NOT be depended on by: `receipts`, `policy` (forbidden by ADR_PLAYBOOK.md)
//!
//! **Policy re-exports**: This crate re-exports policy validation functions for convenience,
//! but policy validation lives in `northroot-policy` to maintain clear boundaries.

pub mod api;
pub mod cas;
pub mod commitments;
pub mod composition;
pub mod delta;
pub mod execution;
pub mod serde_helpers;
pub mod shapes;
pub mod signature;

pub use commitments::{
    cbor_deterministic, cbor_hash, commit_seq_root, commit_set_root, jcs, sha256_prefixed,
    validate_cbor_deterministic,
};
pub use composition::{
    build_sequential_chain, compute_tensor_root, create_identity_receipt, validate_all_links,
    validate_link, validate_sequential, CompositionError,
};
// Re-export policy items for convenience.
//
// **Boundary note**: Policy validation lives in `northroot-policy` crate.
// Engine re-exports these for convenience, but policy validation is NOT part of engine.
// Policy answers "is this allowed?" (semantic validation), not "how do I compute this?" (engine).
pub use northroot_policy::{
    load_policy, validate_determinism, validate_policy, validate_policy_ref_format,
    validate_region_constraints, validate_tool_constraints, PolicyError,
};
pub use signature::{resolve_did_key, verify_all_signatures, verify_signature, SignatureError};

// Re-export delta module items for convenience
pub use delta::{
    chunk_id_from_bytes, chunk_id_from_str, decide_reuse, decide_reuse_with_layer, economic_delta,
    jaccard_similarity, load_cost_model_from_policy, verify_exact_set, weighted_jaccard_similarity,
    ChunkSet, ReuseDecision,
};

// Re-export execution module items
pub use execution::{
    compute_execution_roots, generate_trace_id, validate_method_ref, ExecutionReceiptBuilder,
};

// Re-export cas module items
pub use cas::{
    build_bytestream_manifest, build_manifest_from_data, chunk_by_cdc, chunk_by_fixed,
    deserialize_manifest_from_cbor, estimate_jaccard_from_indices, serialize_manifest_to_cbor,
    BloomParams, ByteStreamManifest, CasError, Chunk, ExternalSetRef, OverlapIndex, SketchData,
};

// Re-export shapes module items
pub use shapes::{
    compute_data_shape_hash, ChunkScheme, DataShape, DataShapeError, KeyFormat, RowValueRepr,
};

// Re-export delta module items for OverlapMetric
pub use delta::OverlapMetric;

// Re-export API module items for SDK
pub use api::{record_work, verify_receipt, ApiError};

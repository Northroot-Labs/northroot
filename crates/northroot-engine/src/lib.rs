//! Northroot Engine: Proof algebra implementation and execution engine.
//!
//! This crate provides the core engine for the Northroot proof algebra system,
//! including receipt composition, validation, and execution tracking.

pub mod commitments;
pub mod composition;
pub mod delta;
pub mod execution;
pub mod policy;
pub mod signature;
pub mod strategies;

pub use commitments::{commit_seq_root, commit_set_root, jcs, sha256_prefixed};
pub use composition::{
    build_sequential_chain, compute_tensor_root, create_identity_receipt, validate_all_links,
    validate_link, validate_sequential, CompositionError,
};
pub use policy::{
    load_policy, validate_determinism, validate_policy, validate_policy_ref_format,
    validate_region_constraints, validate_tool_constraints, PolicyError,
};
pub use signature::{
    resolve_did_key, verify_all_signatures, verify_signature, SignatureError,
};

// Re-export delta module items for convenience
pub use delta::{
    chunk_id_from_bytes, chunk_id_from_str, decide_reuse, decide_reuse_with_layer,
    economic_delta, jaccard_similarity, verify_exact_set, weighted_jaccard_similarity, ChunkSet,
    CostModel, ReuseDecision,
};

// Re-export execution module items
pub use execution::{
    compute_execution_roots, generate_trace_id, validate_method_ref, ExecutionReceiptBuilder,
    MerkleRowMap,
};

// Re-export strategies module items
pub use strategies::{
    ExecutionMode, IncrementalSumStrategy, PartitionStrategy, Strategy, StrategyError,
};

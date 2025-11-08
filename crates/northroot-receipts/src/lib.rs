//! Northroot Receipts — Canonical data model for the proof algebra system.
//!
//! This crate defines the unified receipt envelope and all kind-specific payloads
//! (data_shape, method_shape, reasoning_shape, execution, spend, settlement) as Rust types
//! with JSON Schema validation.
//!
//! # Features
//!
//! - `jsonschema`: Enable runtime JSON Schema validation (for untrusted input)
//! - `builders`: Enable convenience builder APIs for constructing receipts

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

pub mod canonical;
pub mod error;
pub mod receipt_deser;
#[cfg(feature = "jsonschema")]
pub mod schema;
// Test utilities module - available for test modules
// Note: This module is only used for generating test vectors
// and is not part of the public API
pub mod test_utils;
pub mod validation;

pub use canonical::{canonical_body, compute_hash, validate_hash_format};
pub use error::ValidationError;
#[cfg(feature = "jsonschema")]
pub use schema::validate_payload_schema;
pub use validation::{validate, validate_composition, validate_hash, validate_payload};

// Re-export test utilities for use in test modules
pub use test_utils::{
    generate_data_shape_receipt, generate_execution_receipt, generate_method_shape_receipt,
    generate_reasoning_shape_receipt, generate_sequential_chain, generate_settlement_receipt,
    generate_spend_receipt,
};

// ---------------- Envelope ----------------

/// Receipt kind determines the type of payload and the semantic meaning of the receipt.
///
/// Each kind represents a different stage in the proof algebra pipeline:
/// - Data shapes define input schemas
/// - Method shapes define operator contracts
/// - Reasoning shapes capture decision logic
/// - Execution records observable runs
/// - Spend tracks resource consumption
/// - Settlement handles multi-party netting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptKind {
    /// Data shape receipt: defines input schema and optional statistical sketches
    DataShape,
    /// Method shape receipt: defines operator contracts as multiset/DAG
    MethodShape,
    /// Reasoning shape receipt: captures decision/plan DAG (why)
    ReasoningShape,
    /// Execution receipt: records observable run structure (what/when)
    Execution,
    /// Spend receipt: tracks metered resources and pricing (value/cost)
    Spend,
    /// Settlement receipt: handles multi-party netting (result)
    Settlement,
}

/// Determinism class indicates the level of reproducibility guaranteed by a receipt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeterminismClass {
    /// Bit-identical reproducible outputs
    Strict,
    /// Bounded nondeterminism (e.g., float tolerances, seeded RNG)
    Bounded,
    /// Observational log with no reproducibility claim
    Observational,
}

/// Context provides metadata about the receipt: policy, timing, determinism, and identity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Context {
    /// Policy reference in format `pol:<namespace>/<name>@<semver>` (e.g., "pol:standard-v1")
    pub policy_ref: Option<String>,
    /// Timestamp in RFC3339 format (UTC, with milliseconds)
    pub timestamp: String,
    /// Optional nonce (base64url encoded)
    pub nonce: Option<String>,
    /// Determinism class indicating reproducibility guarantees
    pub determinism: Option<DeterminismClass>,
    /// Identity reference as DID URI (e.g., "did:key:z6Mk..." or "did:web:northroot.dev")
    pub identity_ref: Option<String>,
}

/// Signature provides cryptographic proof over the receipt hash.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Signature {
    /// Signature algorithm (e.g., "ed25519")
    pub alg: String,
    /// Key identifier (DID key id)
    pub kid: String,
    /// Signature value (base64url encoded) over the canonical body hash
    pub sig: String,
}

/// Receipt payload: kind-specific data that varies by receipt type.
///
/// The payload type is determined by the `kind` field in the receipt envelope.
#[derive(Debug, Clone, PartialEq)]
pub enum Payload {
    /// Data shape payload
    DataShape(DataShapePayload),
    /// Method shape payload
    MethodShape(MethodShapePayload),
    /// Reasoning shape payload
    ReasoningShape(ReasoningShapePayload),
    /// Execution payload
    Execution(ExecutionPayload),
    /// Spend payload
    Spend(SpendPayload),
    /// Settlement payload
    Settlement(SettlementPayload),
}

impl serde::Serialize for Payload {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as the inner payload value (not as a tagged enum)
        match self {
            Payload::DataShape(p) => p.serialize(serializer),
            Payload::MethodShape(p) => p.serialize(serializer),
            Payload::ReasoningShape(p) => p.serialize(serializer),
            Payload::Execution(p) => p.serialize(serializer),
            Payload::Spend(p) => p.serialize(serializer),
            Payload::Settlement(p) => p.serialize(serializer),
        }
    }
}

impl<'de> serde::Deserialize<'de> for Payload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let value = serde_json::Value::deserialize(deserializer)?;

        // Try to determine payload type from structure
        // This is a fallback - Receipt deserializer should handle it properly
        if value.get("schema_hash").is_some() {
            Ok(Payload::DataShape(
                serde_json::from_value(value).map_err(Error::custom)?,
            ))
        } else if value.get("nodes").is_some() {
            Ok(Payload::MethodShape(
                serde_json::from_value(value).map_err(Error::custom)?,
            ))
        } else if value.get("intent_hash").is_some() {
            Ok(Payload::ReasoningShape(
                serde_json::from_value(value).map_err(Error::custom)?,
            ))
        } else if value.get("trace_id").is_some() {
            Ok(Payload::Execution(
                serde_json::from_value(value).map_err(Error::custom)?,
            ))
        } else if value.get("meter").is_some() {
            Ok(Payload::Spend(
                serde_json::from_value(value).map_err(Error::custom)?,
            ))
        } else if value.get("wur_refs").is_some() {
            Ok(Payload::Settlement(
                serde_json::from_value(value).map_err(Error::custom)?,
            ))
        } else {
            Err(Error::custom("Cannot determine payload type"))
        }
    }
}

/// Unified receipt envelope
///
/// A receipt is the signed evidence of a morphism in the proof algebra.
/// All receipts share this common envelope structure, with kind-specific payloads.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Receipt {
    /// Receipt ID (UUIDv7 recommended for time-ordered IDs)
    pub rid: Uuid,
    /// Envelope version (e.g., "0.3.0")
    pub version: String,
    /// Receipt kind (determines payload type)
    pub kind: ReceiptKind,
    /// Domain shape commitment: sha256:<64hex>
    pub dom: String,
    /// Codomain shape commitment: sha256:<64hex>
    pub cod: String,
    /// Child receipts for composition (optional)
    #[serde(default)]
    pub links: Vec<Uuid>,
    /// Context: policy, timestamp, determinism, identity
    pub ctx: Context,
    /// Kind-specific payload
    pub payload: Payload,
    /// Optional TEE/container attestations
    pub attest: Option<serde_json::Value>,
    /// Detached signature over `hash` (optional)
    pub sig: Option<Signature>,
    /// SHA-256 hash of canonical body (without sig/hash)
    pub hash: String,
}

impl Receipt {
    /// Compute hash from canonical body.
    ///
    /// This computes the SHA-256 hash of the canonical JSON representation
    /// (without `sig` and `hash` fields) and returns it in the format `sha256:<64hex>`.
    pub fn compute_hash(&self) -> Result<String, ValidationError> {
        canonical::compute_hash(self)
            .map_err(|e| ValidationError::SerializationError(e.to_string()))
    }

    /// Fast validation: hash integrity, payload rules, and format checks (no JSON Schema).
    ///
    /// This is the default validation that uses Rust type checking and custom validators.
    /// It's faster than `validate_strict()` and suitable for trusted, in-process paths.
    ///
    /// Returns `Ok(())` if the receipt is valid, or a `ValidationError` if validation fails.
    pub fn validate_fast(&self) -> Result<(), ValidationError> {
        validation::validate(self)
    }

    /// Strict validation: includes JSON Schema validation (requires `jsonschema` feature).
    ///
    /// This performs all checks from `validate_fast()` plus runtime JSON Schema validation.
    /// Use this for untrusted input at API boundaries.
    ///
    /// Returns `Ok(())` if the receipt is valid, or a `ValidationError` if validation fails.
    #[cfg(feature = "jsonschema")]
    pub fn validate_strict(&self) -> Result<(), ValidationError> {
        self.validate_fast()?;
        schema::validate_payload_schema(self)?;
        Ok(())
    }

    /// Validate this receipt (alias for `validate_fast()` for backward compatibility).
    ///
    /// Returns `Ok(())` if the receipt is valid, or a `ValidationError` if validation fails.
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.validate_fast()
    }
}

impl<'de> serde::Deserialize<'de> for Receipt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        receipt_deser::deserialize_receipt(deserializer)
    }
}

// ---------------- Kinds & Payloads ----------------

/// Data shape payload: schema commitment and optional statistical sketches.
///
/// Data shapes define the structure and constraints of input data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataShapePayload {
    /// Schema hash: SHA-256 commitment to the data schema
    pub schema_hash: String,
    /// Optional sketch hash: SHA-256 commitment to statistical summaries (min/max/cardinality)
    pub sketch_hash: Option<String>,
}

/// Method shape payload: operator contracts organized as a multiset/DAG.
///
/// Method shapes define the computational plan: which operators run in what order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MethodShapePayload {
    /// Operator nodes: list of operators with their span shape commitments
    pub nodes: Vec<MethodNodeRef>,
    /// Optional edges: defines execution order/dependencies (DAG structure)
    pub edges: Option<Vec<Edge>>,
    /// Root multiset: SHA-256 hash of sorted span shape hashes (multiset root)
    pub root_multiset: String,
    /// Optional DAG hash: SHA-256 hash of canonical DAG representation
    pub dag_hash: Option<String>,
}

/// Reference to a method node (operator) within a method shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MethodNodeRef {
    /// Node identifier (stable within the method)
    pub id: String,
    /// Span shape hash: SHA-256 commitment to the operator's span shape
    pub span_shape_hash: String,
}

/// Edge in a method shape DAG, representing execution order or data flow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Edge {
    /// Source node ID
    pub from: String,
    /// Target node ID
    pub to: String,
}

/// Reasoning shape payload: decision/plan DAG capturing the "why" of computation.
///
/// Reasoning shapes link high-level intent to concrete operator plans.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReasoningShapePayload {
    /// Intent hash: SHA-256 of canonicalized intent/request
    pub intent_hash: String,
    /// DAG hash: SHA-256 of tool-call DAG
    pub dag_hash: String,
    /// Node references: links to operators and optional execution receipts
    pub node_refs: Vec<ReasoningNodeRef>,
    /// Optional policy reference
    pub policy_ref: Option<String>,
    /// Optional quality metrics for reasoning evaluation
    pub quality: Option<ReasoningQuality>,
}

/// Reference to a reasoning node (plan step) within a reasoning shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReasoningNodeRef {
    /// Plan node identifier
    pub node_id: String,
    /// Operator reference (e.g., "ns.io/read_csv@1")
    pub operator_ref: String,
    /// Optional proof-of-execution reference: RID of execution receipt backing this step
    pub pox_ref: Option<String>,
}

/// Quality metrics for reasoning evaluation.
///
/// Note: This struct cannot implement `Eq` because `f32` does not implement `Eq`
/// (floating point comparison is intentionally avoided).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReasoningQuality {
    /// Success score in range [0,1]
    pub success_score: Option<f32>,
    /// Evaluation method identifier
    pub eval_method: Option<String>,
    /// Review hash: SHA-256 of external review document
    pub review_hash: Option<String>,
    /// Confidence score in range [0,1]
    pub confidence: Option<f32>,
}

/// Execution payload: observable run structure capturing "what" and "when".
///
/// Execution receipts record the actual execution of a method over data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionPayload {
    /// Trace identifier for this execution run
    pub trace_id: String,
    /// Method reference: method identifier, version, and shape root
    pub method_ref: MethodRef,
    /// Data shape hash: SHA-256 commitment to the input data shape
    pub data_shape_hash: String,
    /// Span commitments: list of SHA-256 hashes of individual span outputs
    pub span_commitments: Vec<String>,
    /// Execution roots: Merkle roots for set, identity, and sequence structures
    pub roots: ExecutionRoots,
}

/// Reference to a method (operator plan) used in execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MethodRef {
    /// Method identifier (e.g., "com.acme/normalize-ledger")
    pub method_id: String,
    /// Method version (semver format, e.g., "1.0.0")
    pub version: String,
    /// Method shape root: SHA-256 hash of the method shape
    pub method_shape_root: String,
}

/// Execution roots: Merkle roots for different views of span commitments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionRoots {
    /// Trace set root: SHA-256 hash of sorted span commitments (set view)
    pub trace_set_root: String,
    /// Identity root: Merkle root over identity records (DID, kid, role, tenant)
    /// 
    /// This root commits to the set of actors/keys that participated in the execution,
    /// independent of trace ordering. See `identity_root_from_identities` for computation.
    pub identity_root: String,
    /// Optional trace sequence root: SHA-256 hash of ordered span commitments (sequence view)
    pub trace_seq_root: Option<String>,
}

/// Spend payload: metered resources, pricing, and cost accounting.
///
/// Spend receipts track resource consumption and economic value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpendPayload {
    /// Resource quantities (meter readings)
    pub meter: ResourceVector,
    /// Unit prices per resource type
    pub unit_prices: ResourceVector,
    /// Currency code (ISO-4217, 3 characters, e.g., "USD")
    pub currency: String,
    /// Optional pricing policy reference
    pub pricing_policy_ref: Option<String>,
    /// Total value: dot product of meter and unit_prices (with small floating-point tolerance)
    pub total_value: f64,
    /// Pointers linking back to the execution that generated this spend
    pub pointers: SpendPointers,
    /// Optional reuse justification for delta compute decisions
    pub justification: Option<ReuseJustification>,
}

/// Resource vector: metered resource quantities or unit prices.
///
/// All fields are optional; only present resources are tracked.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceVector {
    /// Virtual CPU seconds
    pub vcpu_sec: Option<f64>,
    /// GPU seconds
    pub gpu_sec: Option<f64>,
    /// Memory (GB-seconds)
    pub gb_sec: Option<f64>,
    /// Number of requests
    pub requests: Option<f64>,
    /// Energy consumption (kWh)
    pub energy_kwh: Option<f64>,
}

/// Spend pointers: links back to the execution that generated the spend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpendPointers {
    /// Trace ID of the execution
    pub trace_id: String,
    /// Optional list of specific span IDs within the trace
    pub span_ids: Option<Vec<String>>,
}

/// Reuse justification for delta compute decisions.
///
/// Records the economic and technical parameters that justify a reuse vs recompute decision.
/// The `layer` field indicates the semantic level of shape equivalence at which the reuse
/// decision is evaluated and justified.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReuseJustification {
    /// Jaccard overlap [0,1] between prior and current chunk sets
    pub overlap_j: Option<f64>,
    /// Operator incrementality factor [0,1] - how efficiently deltas can be applied
    pub alpha: Option<f64>,
    /// Identity/integration cost - cost to identify overlap, validate, and splice reused results
    pub c_id: Option<f64>,
    /// Avoided compute cost baseline - baseline cost to (re)execute operator
    pub c_comp: Option<f64>,
    /// Reuse decision: "reuse" | "recompute" | "hybrid"
    pub decision: Option<String>,
    /// Layer (delta evaluation scope): semantic level of shape equivalence for reuse decision.
    ///
    /// Allowed values (v0.1):
    /// - "data": Raw or structured data inputs (chunk IDs, partitions, file blocks)
    /// - "method": Operator/method plans (method shape roots, operator parameters)
    /// - "reasoning": Logical or semantic plans (reasoning DAGs, policy references)
    /// - "execution": Observed span outputs (span commitments, trace roots)
    ///
    /// Purpose: Audit transparency, policy scoping, economic clarity.
    pub layer: Option<String>,
}

/// Settlement payload: multi-party netting and clearing.
///
/// Settlement receipts aggregate multiple work units and compute net positions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SettlementPayload {
    /// Work unit references: list of receipt IDs being settled
    pub wur_refs: Vec<String>,
    /// Net positions: map from party identifier to net amount (positive = owed, negative = owing)
    pub net_positions: BTreeMap<String, f64>,
    /// Rules reference: identifier for the clearing policy/rules used
    pub rules_ref: String,
    /// Optional cash instructions: payment routing details (ACH, stablecoin, credits, etc.)
    pub cash_instr: Option<serde_json::Value>,
}

// ---------------- Identity Root Computation ----------------

/// Identity record for computing identity_root.
///
/// Represents a single actor/key that participated in an execution.
/// Used to compute the Merkle root over all identities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityRecord {
    /// DID (Decentralized Identifier) of the identity
    /// Example: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"
    pub did: String,
    /// Key identifier (kid) within the DID
    /// Example: "did:key:z6Mk...#key-1"
    pub kid: String,
    /// Optional role: "executor", "submitter", "auditor", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Optional tenant identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

/// Compute identity root from a collection of identity records.
///
/// This implements a Merkle tree over identity records using RFC-6962 style domain separation:
/// - Leaf hash = H(0x00 || JCS(identity_record))
/// - Parent hash = H(0x01 || left || right)
/// - Odd nodes are promoted upward (standard CT-tree rule)
/// - Empty tree root = H(0x00 || "")
///
/// Leaves are sorted by (did, kid) lexicographically before hashing.
///
/// # Arguments
///
/// * `identities` - Iterator over identity records
///
/// # Returns
///
/// SHA-256 hash in format `sha256:<64hex>`
///
/// # Example
///
/// ```rust
/// use northroot_receipts::{IdentityRecord, identity_root_from_identities};
///
/// let identities = vec![
///     IdentityRecord {
///         did: "did:key:zNorthroot".to_string(),
///         kid: "did:key:zNorthroot#key-1".to_string(),
///         role: Some("executor".to_string()),
///         tenant: None,
///     },
/// ];
///
/// let root = identity_root_from_identities(identities.into_iter());
/// ```
pub fn identity_root_from_identities<I>(identities: I) -> String
where
    I: Iterator<Item = IdentityRecord>,
{
    use sha2::{Digest, Sha256};

    // Collect and sort identities by (did, kid)
    let mut sorted: Vec<IdentityRecord> = identities.collect();
    sorted.sort_by(|a, b| {
        match a.did.cmp(&b.did) {
            std::cmp::Ordering::Equal => a.kid.cmp(&b.kid),
            other => other,
        }
    });

    // Empty tree: H(0x00 || "")
    if sorted.is_empty() {
        let mut hasher = Sha256::new();
        hasher.update(&[0x00u8]);
        hasher.update(b"");
        return format!("sha256:{:x}", hasher.finalize());
    }

    // Compute leaf hashes: H(0x00 || JCS(identity_record))
    let mut leaf_hashes: Vec<[u8; 32]> = Vec::new();
    for identity in &sorted {
        // Canonicalize identity record with JCS
        let canonical = serde_json::to_string(identity).unwrap();
        
        // Leaf hash = H(0x00 || canonical_bytes)
        let mut hasher = Sha256::new();
        hasher.update(&[0x00u8]);
        hasher.update(canonical.as_bytes());
        leaf_hashes.push(hasher.finalize().into());
    }

    // Build Merkle tree: pair hashes, promote odd nodes
    let mut current_level = leaf_hashes;
    while current_level.len() > 1 {
        let mut next_level = Vec::new();
        
        // Pair hashes left-to-right
        for i in (0..current_level.len()).step_by(2) {
            if i + 1 < current_level.len() {
                // Parent = H(0x01 || left || right)
                let mut hasher = Sha256::new();
                hasher.update(&[0x01u8]);
                hasher.update(&current_level[i]);
                hasher.update(&current_level[i + 1]);
                next_level.push(hasher.finalize().into());
            } else {
                // Odd node: promote upward unchanged
                next_level.push(current_level[i]);
            }
        }
        
        current_level = next_level;
    }

    // Root is the final remaining hash
    let root_bytes = current_level[0];
    let hex_str: String = root_bytes.iter().map(|b| format!("{:02x}", b)).collect();
    format!("sha256:{}", hex_str)
}

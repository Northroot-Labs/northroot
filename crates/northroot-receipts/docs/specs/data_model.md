Receipt schemas : 

Northroot Receipts — Canonical Data Model (v0.1)

This defines the unified receipt envelope and the typed payloads for each core shape:
data_shape, method_shape, reasoning_shape, execution, spend, settlement.

Use these types in the northroot-receipts crate. The engine will import them later.

⸻

Rust types (source of truth)

// src/lib.rs

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// ---------------- Envelope ----------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptKind {
    DataShape,
    MethodShape,
    ReasoningShape,
    Execution,
    Spend,
    Settlement,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeterminismClass {
    Strict,        // bit-identical reproducible
    Bounded,       // bounded nondeterminism (e.g., float tolerances)
    Observational, // observational log (no reproducibility claim)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Context {
    pub policy_ref: Option<String>,   // e.g., "pol:standard-v1"
    pub timestamp: String,            // RFC3339 UTC (ms)
    pub nonce: Option<String>,        // base64url
    pub determinism: Option<DeterminismClass>,
    pub identity_ref: Option<String>, // e.g., "did:key:..."
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Signature {
    pub alg: String, // "ed25519"
    pub kid: String, // DID key id
    pub sig: String, // base64url over canonical body hash
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum Payload {
    DataShape(DataShapePayload),
    MethodShape(MethodShapePayload),
    ReasoningShape(ReasoningShapePayload),
    Execution(ExecutionPayload),
    Spend(SpendPayload),
    Settlement(SettlementPayload),
}

/// Unified receipt envelope
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Receipt {
    pub rid: Uuid,              // UUIDv7 recommended
    pub version: String,        // envelope version, e.g., "0.3.0"
    pub kind: ReceiptKind,
    pub dom: String,            // sha256:<64hex> commitment of domain shape
    pub cod: String,            // sha256:<64hex> commitment of codomain shape
    #[serde(default)]
    pub links: Vec<Uuid>,       // child receipts for composition (optional)
    pub ctx: Context,
    pub payload: Payload,
    pub attest: Option<serde_json::Value>, // optional TEE/container attest
    pub sig: Option<Signature>,  // detached signature over `hash`
    pub hash: String,            // sha256 of canonical body (without sig/hash)
}

/// ---------------- Kinds & Payloads ----------------

/// 1) Data shape: schema + optional sketches (stats)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataShapePayload {
    pub schema_hash: String,        // sha256:...
    pub sketch_hash: Option<String> // sha256:...
}

/// 2) Method shape: operator contracts as multiset/DAG
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MethodShapePayload {
    pub nodes: Vec<MethodNodeRef>, // [{id, span_shape_hash}]
    pub edges: Option<Vec<Edge>>,  // optional in v0.2/v0.1 of algebra
    pub root_multiset: String,     // sha256(sorted(span_shape_hash))
    pub dag_hash: Option<String>,  // sha256(canonical DAG)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MethodNodeRef {
    pub id: String,                 // node id (stable within method)
    pub span_shape_hash: String,    // sha256:...
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Edge { pub from: String, pub to: String }

/// 3) Reasoning shape: decision/plan DAG (why)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReasoningShapePayload {
    pub intent_hash: String,             // sha256 of canonicalized intent/request
    pub dag_hash: String,                // sha256 of tool-call DAG
    pub node_refs: Vec<ReasoningNodeRef>,// links to operators (and optional exec refs)
    pub policy_ref: Option<String>,
    pub quality: Option<ReasoningQuality>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReasoningNodeRef {
    pub node_id: String,     // plan node id
    pub operator_ref: String,// e.g., "ns.io/read_csv@1"
    pub pox_ref: Option<String>, // RID of an execution receipt backing this step (if any)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReasoningQuality {
    pub success_score: Option<f32>, // [0,1]
    pub eval_method: Option<String>,
    pub review_hash: Option<String>,// sha256 of external review
    pub confidence: Option<f32>,    // [0,1]
}

/// 4) Execution shape: observable run structure (what/when)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionPayload {
    pub trace_id: String,
    pub method_ref: MethodRef,      // method id@ver + shape root
    pub data_shape_hash: String,    // sha256:...
    pub span_commitments: Vec<String>, // [sha256:...] of span commitments
    pub roots: ExecutionRoots,      // set/identity/sequence roots
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MethodRef {
    pub method_id: String,          // "com.acme/normalize-ledger"
    pub version: String,            // "1.0.0"
    pub method_shape_root: String,  // sha256:...
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionRoots {
    pub trace_set_root: String,     // sha256 of sorted span commitments
    pub identity_root: String,      // sha256 of {trace_id, identity, sorted(C_span)}
    pub trace_seq_root: Option<String>, // sha256 of ordered span commitments (optional)
}

/// 5) Spend shape: metered resources + pricing (value/cost)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpendPayload {
    pub meter: ResourceVector,          // resource quantities
    pub unit_prices: ResourceVector,    // price per unit
    pub currency: String,               // ISO-4217 (e.g., "USD")
    pub pricing_policy_ref: Option<String>,
    pub total_value: f64,               // dot(meter, unit_prices) ± ε
    pub pointers: SpendPointers,        // links back to execution
    pub justification: Option<ReuseJustification>, // delta compute hook
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceVector {
    pub vcpu_sec: Option<f64>,
    pub gpu_sec: Option[f64],
    pub gb_sec: Option<f64>,
    pub requests: Option<f64>,
    pub energy_kwh: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpendPointers {
    pub trace_id: String,
    pub span_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReuseJustification {
    pub overlap_j: Option<f64>, // [0,1]
    pub alpha: Option<f64>,     // operator incrementality factor
    pub c_id: Option<f64>,      // identity/integration cost
    pub c_comp: Option<f64>,    // avoided compute cost baseline
    pub decision: Option<String>, // "reuse" | "recompute" | "hybrid"
}

/// 6) Settlement shape: multi-party netting (result)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SettlementPayload {
    pub wur_refs: Vec<String>,                 // work-unit/receipt references
    pub net_positions: BTreeMap<String, f64>,  // party -> amount (+/-)
    pub rules_ref: String,                     // clearing policy/rules
    pub cash_instr: Option<serde_json::Value>, // ACH/stablecoin/credits routing
}

Notes:
	•	All hashes are sha256:<64hex>.
	•	Timestamps: RFC3339 UTC with millisecond precision.
	•	Arrays that represent sets (e.g., span commitments for set root) are sorted before hashing at the engine level; the receipt stores the resulting root(s).
	•	version is the envelope version; per-kind schema versions live in docs/schemas (JSON Schema).

⸻

JSON Schemas (concise, one per kind)

Put these in schemas/. The envelope is fixed; validators first check envelope, then the kind payload against its schema.

schemas/data_shape.schema.json

{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://northroot.dev/schemas/data_shape.schema.json",
  "title": "DataShapePayload",
  "type": "object",
  "required": ["schema_hash"],
  "properties": {
    "schema_hash": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" },
    "sketch_hash": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" }
  },
  "additionalProperties": false
}

schemas/method_shape.schema.json

{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://northroot.dev/schemas/method_shape.schema.json",
  "title": "MethodShapePayload",
  "type": "object",
  "required": ["nodes", "root_multiset"],
  "properties": {
    "nodes": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "required": ["id", "span_shape_hash"],
        "properties": {
          "id": { "type": "string", "minLength": 1 },
          "span_shape_hash": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" }
        },
        "additionalProperties": false
      }
    },
    "edges": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["from", "to"],
        "properties": {
          "from": { "type": "string" },
          "to":   { "type": "string" }
        },
        "additionalProperties": false
      }
    },
    "root_multiset": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" },
    "dag_hash": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" }
  },
  "additionalProperties": false
}

schemas/reasoning_shape.schema.json

{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://northroot.dev/schemas/reasoning_shape.schema.json",
  "title": "ReasoningShapePayload",
  "type": "object",
  "required": ["intent_hash", "dag_hash", "node_refs"],
  "properties": {
    "intent_hash": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" },
    "dag_hash": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" },
    "node_refs": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["node_id", "operator_ref"],
        "properties": {
          "node_id": { "type": "string" },
          "operator_ref": { "type": "string" },
          "pox_ref": { "type": "string" }
        },
        "additionalProperties": false
      }
    },
    "policy_ref": { "type": "string" },
    "quality": {
      "type": "object",
      "properties": {
        "success_score": { "type": "number", "minimum": 0, "maximum": 1 },
        "eval_method": { "type": "string" },
        "review_hash": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" },
        "confidence": { "type": "number", "minimum": 0, "maximum": 1 }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}

schemas/execution.schema.json

{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://northroot.dev/schemas/execution.schema.json",
  "title": "ExecutionPayload",
  "type": "object",
  "required": ["trace_id", "method_ref", "data_shape_hash", "span_commitments", "roots"],
  "properties": {
    "trace_id": { "type": "string", "minLength": 1 },
    "method_ref": {
      "type": "object",
      "required": ["method_id", "version", "method_shape_root"],
      "properties": {
        "method_id": { "type": "string" },
        "version": { "type": "string" },
        "method_shape_root": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" }
      },
      "additionalProperties": false
    },
    "data_shape_hash": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" },
    "span_commitments": {
      "type": "array",
      "minItems": 1,
      "items": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" }
    },
    "roots": {
      "type": "object",
      "required": ["trace_set_root", "identity_root"],
      "properties": {
        "trace_set_root": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" },
        "identity_root":  { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" },
        "trace_seq_root": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}

schemas/spend.schema.json

{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://northroot.dev/schemas/spend.schema.json",
  "title": "SpendPayload",
  "type": "object",
  "required": ["meter", "unit_prices", "currency", "total_value", "pointers"],
  "properties": {
    "meter": {
      "type": "object",
      "properties": {
        "vcpu_sec": { "type": "number", "minimum": 0 },
        "gpu_sec": { "type": "number", "minimum": 0 },
        "gb_sec": { "type": "number", "minimum": 0 },
        "requests": { "type": "number", "minimum": 0 },
        "energy_kwh": { "type": "number", "minimum": 0 }
      },
      "additionalProperties": false
    },
    "unit_prices": {
      "type": "object",
      "properties": {
        "vcpu_sec": { "type": "number", "minimum": 0 },
        "gpu_sec": { "type": "number", "minimum": 0 },
        "gb_sec": { "type": "number", "minimum": 0 },
        "requests": { "type": "number", "minimum": 0 },
        "energy_kwh": { "type": "number", "minimum": 0 }
      },
      "additionalProperties": false
    },
    "currency": { "type": "string", "minLength": 3, "maxLength": 3 },
    "pricing_policy_ref": { "type": "string" },
    "total_value": { "type": "number" },
    "pointers": {
      "type": "object",
      "required": ["trace_id"],
      "properties": {
        "trace_id": { "type": "string" },
        "span_ids": { "type": "array", "items": { "type": "string" }, "minItems": 1 }
      },
      "additionalProperties": false
    },
    "justification": {
      "type": "object",
      "properties": {
        "overlap_j": { "type": "number", "minimum": 0, "maximum": 1 },
        "alpha":     { "type": "number", "minimum": 0 },
        "c_id":      { "type": "number", "minimum": 0 },
        "c_comp":    { "type": "number", "minimum": 0 },
        "decision":  { "type": "string", "enum": ["reuse", "recompute", "hybrid"] }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}

schemas/settlement.schema.json

{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://northroot.dev/schemas/settlement.schema.json",
  "title": "SettlementPayload",
  "type": "object",
  "required": ["wur_refs", "net_positions", "rules_ref"],
  "properties": {
    "wur_refs": {
      "type": "array",
      "minItems": 1,
      "items": { "type": "string" }
    },
    "net_positions": {
      "type": "object",
      "additionalProperties": { "type": "number" }
    },
    "rules_ref": { "type": "string" },
    "cash_instr": { "type": "object" }
  },
  "additionalProperties": false
}


⸻

Validation invariants (engine will enforce)
	•	hash = sha256(canonical body without sig/hash).
	•	Every sha256: field matches ^sha256:[0-9a-f]{64}$.
	•	execution.span_commitments non-empty; roots present/valid.
	•	spend.total_value ≈ dot(meter, unit_prices) within ε.
	•	Sequential composition: cod(prev) == dom(next) (checked when linking receipts).
	•	Determinism class honored by policy (if set).
	•	Optional Merkle/link integrity for composed graphs.

⸻
# CORE_EVENTS.md

## 0. Purpose

Northroot emits **Verifiable Events**: append-only, audit-grade evidence records
that are canonicalized, hash-identifiable, hygiene-checked, policy-bound, and
optionally attested.

Northroot is intentionally **neutral**:
- It standardizes the **evidence format** and **verification rules**.
- It does NOT mandate any specific policy language, enforcement engine, agent framework,
  orchestration model, or runtime semantics.

The strongest guarantees come when a deployment chooses to **enforce at the boundary**,
but evidence-only mode remains valid.

---

## 1. Definitions

### 1.1 Event
An immutable record in an append-only log.

### 1.2 Verifiable Event
An event with:
- canonical JSON payload bytes (per canonicalization profile)
- `event_id` derived from a digest of canonical bytes
- explicit linkage to intents, policies, and (optionally) prior events
- hygiene evidence for any inputs that influence policy or execution

### 1.3 Principal
The responsible actor for an action. Can be a human, service, agent, or org.
Northroot models all of these uniformly as `PrincipalID`.

### 1.4 Policy
Any decision-making mechanism used to allow/deny actions and set bounds.
Northroot does not constrain policy language or engine.
Policy identity is bound by `policy_id` and `policy_digest`.

### 1.5 Tool
A boundary-callable interface: LLM API, database query function, HTTP fetch, MCP call,
filesystem operation, etc. Tool identity is by stable `tool_name`.

### 1.6 Intent
A canonical description of what is being requested/evaluated.
Intent is the proof anchor for “what policy evaluated” and “what executed”.

---

## 2. Core invariants

### E1 — Append-only
Events are immutable once written. Corrections are represented as new events.

### E2 — Stable identity
Each event has:
- `event_id = Digest(canonical_bytes(event_payload))`

### E3 — Canonicalization binding
Each event binds:
- `canonical_profile_id` (ProfileID)
- `payload_digest` (Digest) of canonical bytes
Verification MUST recompute digests from canonicalization rules.

### E4 — Hygiene binding
If any part of intent or inputs influence policy or execution, hygiene must be recorded
(either inline or by digest/reference).

### E5 — Policy binding
Authorization events must bind:
- `policy_id`
- `policy_digest`
- a decision (`allow|deny`)
- reason codes and bounds (when allow)

### E6 — Neutral cost model
Cost and limits are expressed as a **meter vector**:
- `[{ unit: string, amount: Quantity }]`

No single “money” or “tokens” assumption exists in core. Units are namespaced strings.

### E7 — Optional opaque resources
Resource modeling is optional and opaque. Resources are never required for core verification.

### E8 — Offline verifiability
A verifier must be able to validate:
- canonicalization + digests
- event link consistency
- bounds vs metering
without network calls.

---

## 3. Event envelope

All events share a common envelope:

- `event_type`: string (e.g., "authorization", "execution")
- `event_version`: string (e.g., "1")
- `event_id`: Digest
- `occurred_at`: Timestamp
- `principal_id`: PrincipalID
- `canonical_profile_id`: ProfileID
- `payload_digest`: Digest (digest of canonical bytes of the event payload)
- `request_id`: optional string (operational correlation only)
- `context_digest`: optional Digest (opaque, for ecosystem-specific metadata)

`event_id` and `payload_digest` may be identical if `event_id` is defined as digest of the
canonical payload bytes. If both exist, they MUST be consistent per profile rules.

---

## 4. Core event types (v0)

### 4.1 AuthorizationEvent

Authorization is the pre-action policy decision.

Authorization has two kinds:
- `kind = "grant"`: a capability envelope (tool allowlists, budgets, TTL, etc.)
- `kind = "action"`: authorization for a specific tool call

Both kinds use:
- `decision`: "allow" | "deny"
- `decision_code`: stable string (e.g., "ALLOW", "SPEND_CAP_EXCEEDED")
- `checks`: optional list of structured check outcomes (tool, hygiene, spend, rate, time)

Authorization MUST bind:
- `intent_digest` (what was evaluated)
- `policy_id`, `policy_digest` (what decided)

If `decision == "allow"`, authorization MUST include explicit bounds.

#### Grant bounds (recommended fields)
- `expires_at`: Timestamp
- `allowed_tools`: list of ToolName or prefixes (namespace allowlist)
- `meter_caps`: meter vector caps
- `rate_limits`: optional (calls per window)
- `concurrency_limit`: optional Int
- `output_mode`: "digest_only" | "allow_inline" | "allow_contentref"
- `resources`: optional opaque selectors

#### Action authorization fields
- `tool_name`: ToolName (required)
- `tool_params_digest`: Digest (required)
- `meter_reservation`: optional meter vector (reserved budget slice)

Action authorization MAY reference a prior grant authorization via `grant_event_id`.

---

### 4.2 ExecutionEvent

Execution is post-action evidence.

Execution MUST bind:
- `auth_event_id` (the AuthorizationEvent that permitted/denied this execution attempt)
- `intent_digest` (must match authorization)
- `tool_name`
- `meter_used`: meter vector
- `outcome`: "success" | "failure"
- `error_code`: optional stable string
- `output_digest`: optional Digest
- `output_ref`: optional ContentRef

If execution occurs, it MUST have a corresponding authorization reference.
If a system executes without enforcement, it must still emit the authorization decision
as evidence (evidence-only mode).

---

## 5. Enforcement is optional; evidence is mandatory

Deployments may choose:

- Evidence-only mode: internal enforcement stays in existing systems; Northroot records
  verifiable events about decisions and executions.

- Boundary enforcement mode: a gateway evaluates policy and blocks or permits tool calls,
  emitting authorization and execution events. This yields strongest guarantees.

Northroot does not prescribe the policy engine or workflow, only the evidence format and
verification rules.

---


⸻

Rust API surface draft (neutral, enforcement-pluggable)

/// Stable, receipt-grade event model: append-only + verifiable.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileId(pub String); // validated elsewhere

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolName(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipalId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Digest {
    pub alg: String, // e.g. "sha-256"
    pub b64: String, // base64url no-pad
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRef {
    pub digest: Digest,
    pub size_bytes: Option<u64>,
    pub media_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timestamp(pub String); // RFC3339 Z; validate elsewhere

// ---- Meter model (neutral cost vector) ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meter {
    pub unit: String,     // namespaced string: "usd", "tokens.input", "http.requests"
    pub amount: Quantity, // canonical numeric types
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum Quantity {
    #[serde(rename = "dec")]
    Dec { m: String, s: u32 },
    #[serde(rename = "int")]
    Int { v: String },
    #[serde(rename = "rat")]
    Rat { n: String, d: String },
    #[serde(rename = "f64")]
    F64 { bits: String },
}

// ---- Hygiene ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HygieneStatus { Ok, Lossy, Ambiguous, Invalid }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HygieneReport {
    pub status: HygieneStatus,
    pub warnings: Vec<String>,
    pub metrics: std::collections::BTreeMap<String, u64>,
    pub profile_id: String,
}

// ---- Event envelope ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_type: String,          // "authorization" | "execution" | ...
    pub event_version: String,       // "1"
    pub event_id: Digest,            // digest of canonical payload bytes
    pub occurred_at: Timestamp,
    pub principal_id: PrincipalId,
    pub canonical_profile_id: ProfileId,
    pub payload_digest: Digest,      // digest of canonical payload bytes (may equal event_id)
    pub request_id: Option<String>,  // operational correlation only
    pub context_digest: Option<Digest>, // opaque ecosystem context
}

// ---- Intent anchors ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentAnchors {
    pub intent_digest: Digest,             // required spine
    pub intent_ref: Option<ContentRef>,    // optional content-addressed payload
    pub user_intent_digest: Option<Digest> // optional higher-level task intent (human or upstream agent)
}

// ---- Authorization ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decision { Allow, Deny }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub check: String,        // "tool", "hygiene", "spend", "rate", "time", ...
    pub result: String,       // "pass" | "fail"
    pub code: Option<String>, // stable reason code for this check
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputMode { DigestOnly, AllowInline, AllowContentRef }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantBounds {
    pub expires_at: Option<Timestamp>,
    pub allowed_tools: Vec<String>,        // ToolName or prefix patterns; validate in policy layer
    pub meter_caps: Vec<Meter>,            // caps by unit
    pub rate_limits: Option<Vec<Meter>>,   // optional: e.g. {unit:"calls.per_min", amount:Int}
    pub concurrency_limit: Option<Quantity>, // Int recommended
    pub output_mode: Option<OutputMode>,
    pub resources: Option<Vec<ResourceRef>>, // optional opaque
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionBounds {
    pub tool_name: ToolName,
    pub tool_params_digest: Digest,
    pub meter_reservation: Option<Vec<Meter>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRef {
    pub kind: String,    // opaque: "db.table", "s3.object", "http.endpoint"
    pub reference: String, // opaque selector
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum AuthorizationKind {
    #[serde(rename = "grant")]
    Grant { bounds: GrantBounds },

    #[serde(rename = "action")]
    Action {
        grant_event_id: Option<Digest>, // links to a grant authorization if present
        action: ActionBounds
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationEvent {
    pub envelope: EventEnvelope,
    pub intents: IntentAnchors,
    pub policy_id: String,
    pub policy_digest: Digest,

    pub decision: Decision,
    pub decision_code: String,       // REQUIRED stable code
    pub checks: Option<Vec<CheckResult>>,

    pub hygiene: Option<HygieneReport>, // recommended inline for policy gating
    pub authorization: AuthorizationKind
}

// ---- Execution ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Outcome { Success, Failure }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub envelope: EventEnvelope,
    pub intents: IntentAnchors,

    pub auth_event_id: Digest,   // REQUIRED linkage
    pub tool_name: ToolName,

    pub started_at: Option<Timestamp>,
    pub ended_at: Option<Timestamp>,

    pub meter_used: Vec<Meter>,  // REQUIRED
    pub outcome: Outcome,
    pub error_code: Option<String>,

    pub output_digest: Option<Digest>,
    pub output_ref: Option<ContentRef>,

    pub resources_touched: Option<Vec<ResourceRef>>
}

// ---- Enforcement hook (pluggable) ----

pub struct AuthorizationInput {
    pub principal_id: PrincipalId,
    pub tool_name: ToolName,
    pub intent_digest: Digest,
    pub tool_params_digest: Digest,
    pub meters_requested: Option<Vec<Meter>>,
    pub now: Timestamp,
    pub context_digest: Option<Digest>,
    pub hygiene: Option<HygieneReport>,
}

pub struct AuthorizationDecision {
    pub decision: Decision,
    pub decision_code: String,
    pub checks: Vec<CheckResult>,
    pub bounds_delta: Option<Vec<Meter>>, // optional remaining/allocated adjustments
}

pub trait PolicyEvaluator {
    fn evaluate(&self, input: AuthorizationInput) -> AuthorizationDecision;
}
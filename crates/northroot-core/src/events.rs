use northroot_canonical::{
    ContentRef, Digest, HygieneReport, PrincipalId, ProfileId, Timestamp, ToolName,
};
use serde::{Deserialize, Serialize};

use crate::shared::{IntentAnchors, Meter, ResourceRef};

/// Authorization decision: allow or deny.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    /// Action is allowed.
    Allow,
    /// Action is denied.
    Deny,
}

/// Outcome of an execution: success or failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// Execution succeeded.
    Success,
    /// Execution failed.
    Failure,
}

/// Result of a policy check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckResult {
    /// Check identifier (e.g., "tool", "hygiene", "spend", "rate", "time").
    pub check: String,
    /// Result: "pass" or "fail".
    #[serde(rename = "result")]
    pub result: CheckResultStatus,
    /// Optional stable reason code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Status of a check result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckResultStatus {
    /// Check passed.
    Pass,
    /// Check failed.
    Fail,
}

/// Output mode for execution results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    /// Only digest is included.
    DigestOnly,
    /// Inline content is allowed.
    AllowInline,
    /// Content reference is allowed.
    AllowContentref,
}

/// Grant bounds: capability envelope with tool allowlists, budgets, TTL, etc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrantBounds {
    /// Optional expiration timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<Timestamp>,
    /// Allowed tool names or prefixes.
    pub allowed_tools: Vec<String>,
    /// Meter capacity caps.
    pub meter_caps: Vec<Meter>,
    /// Optional rate limits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limits: Option<Vec<Meter>>,
    /// Optional concurrency limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_limit: Option<northroot_canonical::Quantity>,
    /// Optional output mode restriction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_mode: Option<OutputMode>,
    /// Optional resource selectors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Vec<ResourceRef>>,
}

/// Action bounds: authorization for a specific tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionBounds {
    /// Tool name being authorized.
    pub tool_name: ToolName,
    /// Digest of tool parameters.
    pub tool_params_digest: Digest,
    /// Optional meter reservation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_reservation: Option<Vec<Meter>>,
}

/// Authorization kind: grant (capability) or action (specific tool call).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum AuthorizationKind {
    /// Grant authorization: capability envelope.
    #[serde(rename = "grant")]
    Grant {
        /// Grant bounds.
        bounds: GrantBounds,
    },
    /// Action authorization: specific tool call.
    #[serde(rename = "action")]
    Action {
        /// Optional reference to prior grant authorization.
        #[serde(skip_serializing_if = "Option::is_none")]
        grant_event_id: Option<Digest>,
        /// Action bounds.
        action: ActionBounds,
    },
}

/// Authorization event: pre-action policy decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthorizationEvent {
    /// Event ID (computed from canonical bytes).
    pub event_id: Digest,
    /// Event type: "authorization".
    #[serde(rename = "event_type")]
    pub event_type: String,
    /// Event version: "1".
    #[serde(rename = "event_version")]
    pub event_version: String,
    /// Optional previous event ID for hash chaining.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_event_id: Option<Digest>,
    /// When the authorization occurred.
    pub occurred_at: Timestamp,
    /// Principal requesting authorization.
    pub principal_id: PrincipalId,
    /// Canonicalization profile ID.
    pub canonical_profile_id: ProfileId,
    /// Intent anchors.
    pub intents: IntentAnchors,
    /// Policy identifier.
    pub policy_id: String,
    /// Policy digest.
    pub policy_digest: Digest,
    /// Decision: allow or deny.
    pub decision: Decision,
    /// Stable decision code (e.g., "ALLOW", "SPEND_CAP_EXCEEDED").
    pub decision_code: String,
    /// Optional check results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checks: Option<Vec<CheckResult>>,
    /// Optional hygiene report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hygiene: Option<HygieneReport>,
    /// Authorization kind and bounds.
    pub authorization: AuthorizationKind,
}

/// Execution event: post-action evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionEvent {
    /// Event ID (computed from canonical bytes).
    pub event_id: Digest,
    /// Event type: "execution".
    #[serde(rename = "event_type")]
    pub event_type: String,
    /// Event version: "1".
    #[serde(rename = "event_version")]
    pub event_version: String,
    /// Optional previous event ID for hash chaining.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_event_id: Option<Digest>,
    /// When the execution occurred.
    pub occurred_at: Timestamp,
    /// Principal that executed.
    pub principal_id: PrincipalId,
    /// Canonicalization profile ID.
    pub canonical_profile_id: ProfileId,
    /// Intent anchors.
    pub intents: IntentAnchors,
    /// Authorization event ID that permitted/denied this execution.
    pub auth_event_id: Digest,
    /// Tool name that was executed.
    pub tool_name: ToolName,
    /// Optional execution start time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<Timestamp>,
    /// Optional execution end time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Timestamp>,
    /// Meter usage vector.
    pub meter_used: Vec<Meter>,
    /// Execution outcome.
    pub outcome: Outcome,
    /// Optional error code (required if outcome is Failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// Optional output digest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_digest: Option<Digest>,
    /// Optional output content reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_ref: Option<ContentRef>,
    /// Optional resources touched during execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources_touched: Option<Vec<ResourceRef>>,
    /// Optional model identifier for token-to-USD conversion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    /// Optional provider identifier for token-to-USD conversion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Optional digest of the pricing snapshot used for verification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing_snapshot_digest: Option<Digest>,
}

/// Signature in an attestation event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    /// Signature algorithm (e.g., "ed25519", "ecdsa-p256-sha256").
    pub alg: String,
    /// Key identifier (e.g., DID, KMS key ARN, x509 thumbprint).
    pub key_id: String,
    /// Signature bytes (base64url-no-pad).
    pub sig: String,
}

/// Attestation event: signature(s) over a checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestationEvent {
    /// Event ID (computed from canonical bytes).
    pub event_id: Digest,
    /// Event type: "attestation".
    #[serde(rename = "event_type")]
    pub event_type: String,
    /// Event version: "1".
    #[serde(rename = "event_version")]
    pub event_version: String,
    /// Optional previous event ID for hash chaining.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_event_id: Option<Digest>,
    /// When the attestation occurred.
    pub occurred_at: Timestamp,
    /// Principal creating the attestation.
    pub principal_id: PrincipalId,
    /// Canonicalization profile ID.
    pub canonical_profile_id: ProfileId,
    /// Checkpoint event ID being attested.
    pub checkpoint_event_id: Digest,
    /// One or more signatures (1-16 entries).
    pub signatures: Vec<Signature>,
}

/// Window parameters for Merkle root verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleWindow {
    /// Start height of the window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_height: Option<u64>,
    /// End height of the window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_height: Option<u64>,
}

/// Checkpoint event: chain tip and optional Merkle root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointEvent {
    /// Event ID (computed from canonical bytes).
    pub event_id: Digest,
    /// Event type: "checkpoint".
    #[serde(rename = "event_type")]
    pub event_type: String,
    /// Event version: "1".
    #[serde(rename = "event_version")]
    pub event_version: String,
    /// Optional previous event ID for hash chaining.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_event_id: Option<Digest>,
    /// When the checkpoint occurred.
    pub occurred_at: Timestamp,
    /// Principal creating the checkpoint.
    pub principal_id: PrincipalId,
    /// Canonicalization profile ID.
    pub canonical_profile_id: ProfileId,
    /// Latest event ID this checkpoint attests exists in-order.
    pub chain_tip_event_id: Digest,
    /// Monotonic counter for chain tip (number of events since genesis).
    pub chain_tip_height: u64,
    /// Optional Merkle root over a window of events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merkle_root: Option<Digest>,
    /// Optional window parameters (required if merkle_root is present).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<MerkleWindow>,
}


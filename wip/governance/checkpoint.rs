//! Checkpoint event type.

use northroot_canonical::{Digest, PrincipalId, ProfileId, Timestamp};
use serde::{Deserialize, Serialize};

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


//! Attestation event type.

use northroot_canonical::{Digest, PrincipalId, ProfileId, Timestamp};
use serde::{Deserialize, Serialize};

use crate::signature::Signature;

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


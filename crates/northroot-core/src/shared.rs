use northroot_canonical::{ContentRef, Digest, Quantity};
use serde::{Deserialize, Serialize};

/// Intent anchors binding an event to the evaluated/executed intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentAnchors {
    /// Required digest of the intent.
    pub intent_digest: Digest,
    /// Optional content-addressed reference to intent payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_ref: Option<ContentRef>,
    /// Optional higher-level user intent digest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_intent_digest: Option<Digest>,
}

/// Meter entry: unit + amount for cost/resource tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meter {
    /// Unit identifier (e.g., "usd", "tokens.input", "http.requests").
    pub unit: String,
    /// Amount in canonical quantity format.
    pub amount: Quantity,
}

/// Opaque resource reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRef {
    /// Resource kind (e.g., "db.table", "s3.object", "http.endpoint").
    pub kind: String,
    /// Resource reference/selector.
    pub reference: String,
}


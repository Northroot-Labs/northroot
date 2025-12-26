//! Signature type for attestation events.

use serde::{Deserialize, Serialize};

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


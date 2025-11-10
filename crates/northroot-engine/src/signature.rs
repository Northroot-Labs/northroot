//! Signature verification for receipts.
//!
//! This module provides signature verification functionality for receipts,
//! supporting Ed25519 signatures and basic DID key resolution.

use northroot_receipts::{Receipt, Signature};

/// Error types for signature verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureError {
    /// Signature verification failed
    VerificationFailed {
        /// Key identifier that failed
        kid: String,
        /// Reason for failure
        reason: String,
    },
    /// Unsupported signature algorithm
    UnsupportedAlgorithm {
        /// Algorithm that is not supported
        alg: String,
    },
    /// DID resolution failed
    DidResolutionFailed {
        /// DID that could not be resolved
        did: String,
        /// Reason for failure
        reason: String,
    },
    /// Invalid signature format
    InvalidFormat(String),
    /// Missing signature
    MissingSignature,
}

impl std::fmt::Display for SignatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignatureError::VerificationFailed { kid, reason } => {
                write!(
                    f,
                    "Signature verification failed for key {}: {}",
                    kid, reason
                )
            }
            SignatureError::UnsupportedAlgorithm { alg } => {
                write!(f, "Unsupported signature algorithm: {}", alg)
            }
            SignatureError::DidResolutionFailed { did, reason } => {
                write!(f, "DID resolution failed for {}: {}", did, reason)
            }
            SignatureError::InvalidFormat(msg) => {
                write!(f, "Invalid signature format: {}", msg)
            }
            SignatureError::MissingSignature => {
                write!(f, "Missing signature")
            }
        }
    }
}

impl std::error::Error for SignatureError {}

/// Verify a signature over a receipt hash.
///
/// This function verifies that the signature in the receipt is valid for the
/// receipt's hash value. It supports Ed25519 signatures and basic DID key resolution.
///
/// # Arguments
///
/// * `receipt` - Receipt to verify
///
/// # Returns
///
/// `Ok(())` if the signature is valid, or `SignatureError` if verification fails.
///
/// # Note
///
/// Currently supports only Ed25519 signatures and basic `did:key` resolution.
/// Full DID resolution and other signature algorithms are planned for future versions.
///
/// # Example
///
/// ```rust,ignore
/// use northroot_engine::signature::verify_signature;
/// use northroot_receipts::Receipt;
///
/// // Assuming you have a receipt with a signature
/// let receipt: Receipt = /* ... */;
/// verify_signature(&receipt).unwrap();
/// ```
pub fn verify_signature(receipt: &Receipt) -> Result<(), SignatureError> {
    let sig = receipt
        .sig
        .as_ref()
        .ok_or(SignatureError::MissingSignature)?;

    // Verify hash format
    if !receipt.hash.starts_with("sha256:") || receipt.hash.len() != 71 {
        return Err(SignatureError::InvalidFormat(
            "Receipt hash must be in sha256:<64hex> format".to_string(),
        ));
    }

    // Extract hash bytes (skip "sha256:" prefix)
    let hash_hex = &receipt.hash[7..];
    let hash_bytes = hex::decode(hash_hex)
        .map_err(|e| SignatureError::InvalidFormat(format!("Invalid hash hex: {}", e)))?;

    // Verify signature based on algorithm
    match sig.alg.as_str() {
        "ed25519" => verify_ed25519(sig, &hash_bytes),
        alg => Err(SignatureError::UnsupportedAlgorithm {
            alg: alg.to_string(),
        }),
    }
}

/// Verify an Ed25519 signature.
///
/// This function verifies an Ed25519 signature over the receipt hash.
/// It supports basic `did:key` resolution for extracting the public key.
///
/// # Arguments
///
/// * `sig` - Signature structure
/// * `message` - Message bytes (receipt hash)
///
/// # Returns
///
/// `Ok(())` if the signature is valid, or `SignatureError` if verification fails.
fn verify_ed25519(sig: &Signature, message: &[u8]) -> Result<(), SignatureError> {
    use ed25519_dalek::{Signature as Ed25519Signature, Verifier, VerifyingKey};

    // Decode signature bytes (base64url encoding)
    use base64::Engine;
    let sig_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&sig.sig)
        .map_err(|e| {
            SignatureError::InvalidFormat(format!("Invalid base64url signature: {}", e))
        })?;

    if sig_bytes.len() != 64 {
        return Err(SignatureError::InvalidFormat(
            "Ed25519 signature must be 64 bytes".to_string(),
        ));
    }

    let sig_array: [u8; 64] = sig_bytes[..64]
        .try_into()
        .map_err(|_| SignatureError::InvalidFormat("Invalid signature length".to_string()))?;

    let signature = Ed25519Signature::from_bytes(&sig_array);

    // Resolve public key from DID
    let public_key = resolve_did_key(&sig.kid)?;

    // Verify signature
    let verifying_key =
        VerifyingKey::from_bytes(&public_key).map_err(|e| SignatureError::VerificationFailed {
            kid: sig.kid.clone(),
            reason: format!("Invalid public key: {}", e),
        })?;

    verifying_key
        .verify(message, &signature)
        .map_err(|e| SignatureError::VerificationFailed {
            kid: sig.kid.clone(),
            reason: format!("Signature verification failed: {}", e),
        })?;

    Ok(())
}

/// Resolve a DID key to a public key.
///
/// Currently supports basic `did:key` resolution. Full DID resolution
/// with multiple methods is planned for future versions.
///
/// # Arguments
///
/// * `kid` - Key identifier (DID key ID)
///
/// # Returns
///
/// Public key bytes (32 bytes for Ed25519) or `SignatureError` if resolution fails.
pub fn resolve_did_key(kid: &str) -> Result<[u8; 32], SignatureError> {
    // Basic support for did:key format
    // Format: did:key:z<base58-encoded-multibase>
    // For Ed25519, the key is encoded in base58btc with prefix 0xed01

    if !kid.starts_with("did:key:") {
        return Err(SignatureError::DidResolutionFailed {
            did: kid.to_string(),
            reason: "Only did:key format is currently supported".to_string(),
        });
    }

    // Extract the multibase-encoded key (after "did:key:")
    let multibase = &kid[8..];

    if multibase.is_empty() || !multibase.starts_with('z') {
        return Err(SignatureError::DidResolutionFailed {
            did: kid.to_string(),
            reason: "Invalid did:key format: must start with 'z'".to_string(),
        });
    }

    // Decode base58btc (the 'z' prefix indicates base58btc)
    let key_bytes = bs58::decode(&multibase[1..])
        .with_alphabet(bs58::Alphabet::BITCOIN)
        .into_vec()
        .map_err(|e| SignatureError::DidResolutionFailed {
            did: kid.to_string(),
            reason: format!("Base58 decode failed: {}", e),
        })?;

    // For Ed25519, the key should be 34 bytes: 0xed01 (2 bytes) + public key (32 bytes)
    if key_bytes.len() < 34 {
        return Err(SignatureError::DidResolutionFailed {
            did: kid.to_string(),
            reason: format!(
                "Invalid key length: expected at least 34 bytes, got {}",
                key_bytes.len()
            ),
        });
    }

    // Check Ed25519 prefix (0xed01)
    if key_bytes[0] != 0xed || key_bytes[1] != 0x01 {
        return Err(SignatureError::DidResolutionFailed {
            did: kid.to_string(),
            reason: "Key does not appear to be Ed25519 (missing 0xed01 prefix)".to_string(),
        });
    }

    // Extract public key (last 32 bytes)
    let public_key: [u8; 32] = key_bytes[key_bytes.len() - 32..].try_into().map_err(|_| {
        SignatureError::DidResolutionFailed {
            did: kid.to_string(),
            reason: "Failed to extract 32-byte public key".to_string(),
        }
    })?;

    Ok(public_key)
}

/// Verify all signatures on a receipt (if multiple signatures are supported in future).
///
/// Currently, receipts support a single signature. This function is a placeholder
/// for future multi-signature support.
///
/// # Arguments
///
/// * `receipt` - Receipt to verify
///
/// # Returns
///
/// `Ok(())` if all signatures are valid, or `SignatureError` if any verification fails.
pub fn verify_all_signatures(receipt: &Receipt) -> Result<(), SignatureError> {
    verify_signature(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_receipts::{Context, DataShapePayload, Payload, ReceiptKind, Signature};
    use uuid::Uuid;

    fn create_test_receipt_with_sig(hash: String, sig: Option<Signature>) -> Receipt {
        let ctx = Context {
            policy_ref: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            nonce: None,
            determinism: None,
            identity_ref: None,
        };

        let payload = Payload::DataShape(DataShapePayload {
            schema_hash: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                .to_string(),
            sketch_hash: None,
        });

        Receipt {
            rid: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            version: "0.3.0".to_string(),
            kind: ReceiptKind::DataShape,
            dom: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                .to_string(),
            cod: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                .to_string(),
            links: Vec::new(),
            ctx,
            payload,
            attest: None,
            sig,
            hash,
        }
    }

    #[test]
    fn test_verify_signature_missing() {
        let receipt = create_test_receipt_with_sig(
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            None,
        );

        let result = verify_signature(&receipt);
        assert!(result.is_err());
        match result.unwrap_err() {
            SignatureError::MissingSignature => {}
            _ => panic!("Expected MissingSignature error"),
        }
    }

    #[test]
    fn test_verify_signature_invalid_hash_format() {
        let sig = Signature {
            alg: "ed25519".to_string(),
            kid: "did:key:zTest".to_string(),
            sig: "test".to_string(),
        };

        let receipt = create_test_receipt_with_sig("invalid_hash".to_string(), Some(sig));

        let result = verify_signature(&receipt);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_signature_unsupported_algorithm() {
        let sig = Signature {
            alg: "rsa".to_string(),
            kid: "did:key:zTest".to_string(),
            sig: "test".to_string(),
        };

        let receipt = create_test_receipt_with_sig(
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            Some(sig),
        );

        let result = verify_signature(&receipt);
        assert!(result.is_err());
        match result.unwrap_err() {
            SignatureError::UnsupportedAlgorithm { alg } => {
                assert_eq!(alg, "rsa");
            }
            _ => panic!("Expected UnsupportedAlgorithm error"),
        }
    }

    #[test]
    fn test_resolve_did_key_invalid_format() {
        let result = resolve_did_key("not-a-did");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_did_key_missing_z_prefix() {
        let result = resolve_did_key("did:key:invalid");
        assert!(result.is_err());
    }
}

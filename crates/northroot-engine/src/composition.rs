//! Composition operations for receipt chains and graphs.
//!
//! This module provides utilities for composing receipts sequentially and in parallel,
//! validating parent-child relationships, and generating identity receipts.

use northroot_receipts::{Receipt, ReceiptKind, ValidationError};
use uuid::Uuid;

use crate::commitments::commit_set_root;

/// Error types for composition operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositionError {
    /// Sequential composition error: codomain of previous receipt doesn't match domain of next
    SequentialMismatch {
        /// Index of the receipt in the chain where the error occurred
        receipt_index: usize,
        /// Expected codomain hash from previous receipt
        expected_cod: String,
        /// Actual domain hash from next receipt
        actual_dom: String,
    },
    /// Invalid receipt chain: empty or single receipt where multiple expected
    InvalidChain(String),
    /// Parent-child link validation failed
    LinkValidationFailed {
        /// Parent receipt ID
        parent_rid: Uuid,
        /// Child receipt ID that failed validation
        child_rid: Uuid,
        /// Reason for failure
        reason: String,
    },
}

impl std::fmt::Display for CompositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompositionError::SequentialMismatch {
                receipt_index,
                expected_cod,
                actual_dom,
            } => {
                write!(
                    f,
                    "Sequential composition error at receipt {}: expected cod {}, got dom {}",
                    receipt_index, expected_cod, actual_dom
                )
            }
            CompositionError::InvalidChain(msg) => {
                write!(f, "Invalid receipt chain: {}", msg)
            }
            CompositionError::LinkValidationFailed {
                parent_rid,
                child_rid,
                reason,
            } => {
                write!(
                    f,
                    "Link validation failed: parent {} -> child {}: {}",
                    parent_rid, child_rid, reason
                )
            }
        }
    }
}

impl std::error::Error for CompositionError {}

/// Validate sequential composition: `cod(R_i) == dom(R_{i+1})`.
///
/// This ensures that receipts in a chain form a valid sequential morphism where
/// the codomain of each receipt matches the domain of the next.
///
/// Also checks for circular dependencies by ensuring no receipt appears twice in the chain.
///
/// # Arguments
///
/// * `chain` - Slice of receipts in sequential order
///
/// # Returns
///
/// `Ok(())` if the chain is valid, or `CompositionError` if validation fails.
///
/// # Example
///
/// ```rust,no_run
/// use northroot_engine::composition::validate_sequential;
/// use northroot_receipts::Receipt;
///
/// // Assuming you have valid receipts
/// let chain: Vec<Receipt> = vec![/* receipt1, receipt2, receipt3 */];
/// validate_sequential(&chain).unwrap();
/// ```
pub fn validate_sequential(chain: &[Receipt]) -> Result<(), CompositionError> {
    if chain.len() < 2 {
        return Ok(()); // Empty or single receipt is trivially valid
    }

    // Check for circular dependencies: no receipt should appear twice
    let mut seen_rids = std::collections::HashSet::new();
    for receipt in chain.iter() {
        if !seen_rids.insert(receipt.rid) {
            return Err(CompositionError::InvalidChain(format!(
                "Circular dependency detected: receipt {} appears multiple times in chain",
                receipt.rid
            )));
        }
    }

    // Validate sequential composition: cod(R_i) == dom(R_{i+1})
    for i in 0..chain.len().saturating_sub(1) {
        let current_cod = &chain[i].cod;
        let next_dom = &chain[i + 1].dom;

        if current_cod != next_dom {
            return Err(CompositionError::SequentialMismatch {
                receipt_index: i + 1,
                expected_cod: current_cod.clone(),
                actual_dom: next_dom.clone(),
            });
        }
    }

    Ok(())
}

/// Compute tensor (parallel) composition root from child receipt hashes.
///
/// For parallel composition, the tensor commitment is computed as:
/// `C(S_1 ⊗ S_2) = sha256(sorted(C(S_1), C(S_2)) joined with "|")`
///
/// This is order-independent: the same set of receipts produces the same root
/// regardless of ordering.
///
/// # Arguments
///
/// * `child_hashes` - Slice of receipt hashes to compose in parallel
///
/// # Returns
///
/// Tensor root hash in format `sha256:<64hex>`
///
/// # Example
///
/// ```rust
/// use northroot_engine::composition::compute_tensor_root;
///
/// let child_hashes = vec![
///     "sha256:abc123...".to_string(),
///     "sha256:def456...".to_string(),
/// ];
/// let root = compute_tensor_root(&child_hashes);
/// ```
pub fn compute_tensor_root(child_hashes: &[String]) -> String {
    commit_set_root(child_hashes)
}

/// Validate parent-child link relationship.
///
/// Checks that:
/// 1. Child receipt's domain matches parent's codomain (if parent has codomain)
/// 2. Child receipt ID is present in parent's links array
/// 3. Both receipts are valid (hash integrity)
///
/// # Arguments
///
/// * `parent` - Parent receipt
/// * `child` - Child receipt
///
/// # Returns
///
/// `Ok(())` if the link is valid, or `CompositionError` if validation fails.
pub fn validate_link(parent: &Receipt, child: &Receipt) -> Result<(), CompositionError> {
    // Check that child RID is in parent's links
    if !parent.links.contains(&child.rid) {
        return Err(CompositionError::LinkValidationFailed {
            parent_rid: parent.rid,
            child_rid: child.rid,
            reason: format!("Child RID {} not found in parent links", child.rid),
        });
    }

    // Validate that child's domain matches parent's codomain
    if parent.cod != child.dom {
        return Err(CompositionError::LinkValidationFailed {
            parent_rid: parent.rid,
            child_rid: child.rid,
            reason: format!(
                "Domain mismatch: parent cod {} != child dom {}",
                parent.cod, child.dom
            ),
        });
    }

    // Validate hash integrity of both receipts
    if let Err(e) = parent.validate_fast() {
        return Err(CompositionError::LinkValidationFailed {
            parent_rid: parent.rid,
            child_rid: child.rid,
            reason: format!("Parent receipt validation failed: {}", e),
        });
    }

    if let Err(e) = child.validate_fast() {
        return Err(CompositionError::LinkValidationFailed {
            parent_rid: parent.rid,
            child_rid: child.rid,
            reason: format!("Child receipt validation failed: {}", e),
        });
    }

    Ok(())
}

/// Validate all parent-child links in a receipt graph.
///
/// Given a parent receipt and a map of child receipts by RID, validates all
/// parent-child relationships.
///
/// # Arguments
///
/// * `parent` - Parent receipt
/// * `children` - Map from RID to child receipt
///
/// # Returns
///
/// `Ok(())` if all links are valid, or first `CompositionError` encountered.
pub fn validate_all_links(
    parent: &Receipt,
    children: &std::collections::HashMap<Uuid, Receipt>,
) -> Result<(), CompositionError> {
    for child_rid in &parent.links {
        if let Some(child) = children.get(child_rid) {
            validate_link(parent, child)?;
        } else {
            return Err(CompositionError::LinkValidationFailed {
                parent_rid: parent.rid,
                child_rid: *child_rid,
                reason: format!("Child receipt {} not found in children map", child_rid),
            });
        }
    }

    Ok(())
}

/// Create an identity receipt: a no-op receipt where `dom == cod`.
///
/// Identity receipts represent the identity morphism `id_{(S,k)}: (S,k) → (S,k)`.
/// They are useful for composition operations and testing.
///
/// # Arguments
///
/// * `rid` - Receipt ID (UUIDv7 recommended)
/// * `version` - Envelope version (e.g., "0.3.0")
/// * `kind` - Receipt kind
/// * `shape_hash` - Shape commitment (used for both dom and cod)
/// * `ctx` - Context (timestamp, policy, etc.)
///
/// # Returns
///
/// Identity receipt with `dom == cod == shape_hash`
///
/// # Note
///
/// The payload should be minimal/no-op for the given kind. This function creates
/// a basic structure; callers should populate appropriate payload data.
pub fn create_identity_receipt(
    rid: Uuid,
    version: String,
    kind: ReceiptKind,
    shape_hash: String,
    ctx: northroot_receipts::Context,
    payload: northroot_receipts::Payload,
) -> Result<Receipt, ValidationError> {
    let receipt = Receipt {
        rid,
        version,
        kind,
        dom: shape_hash.clone(),
        cod: shape_hash,
        links: Vec::new(),
        ctx,
        payload,
        attest: None,
        sig: None,
        hash: String::new(), // Will be computed
    };

    // Compute hash
    let hash = receipt.compute_hash()?;
    Ok(Receipt { hash, ..receipt })
}

/// Build a sequential chain from individual receipts, validating composition.
///
/// This is a convenience function that validates the chain and returns it if valid.
///
/// # Arguments
///
/// * `receipts` - Vector of receipts in sequential order
///
/// # Returns
///
/// `Ok(Vec<Receipt>)` if the chain is valid, or `CompositionError` if validation fails.
pub fn build_sequential_chain(receipts: Vec<Receipt>) -> Result<Vec<Receipt>, CompositionError> {
    validate_sequential(&receipts)?;
    Ok(receipts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_receipts::{Context, DataShapePayload, Payload, ReceiptKind};
    use uuid::Uuid;

    fn create_test_receipt(
        rid_str: &str,
        dom: String,
        cod: String,
    ) -> Result<Receipt, ValidationError> {
        let rid = Uuid::parse_str(rid_str).unwrap();
        let ctx = Context {
            policy_ref: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            nonce: None,
            determinism: None,
            identity_ref: None,
        };

        let payload = Payload::DataShape(DataShapePayload {
            schema_hash: dom.clone(),
            sketch_hash: None,
        });

        let receipt = Receipt {
            rid,
            version: "0.3.0".to_string(),
            kind: ReceiptKind::DataShape,
            dom,
            cod,
            links: Vec::new(),
            ctx,
            payload,
            attest: None,
            sig: None,
            hash: String::new(),
        };

        let hash = receipt.compute_hash()?;
        Ok(Receipt { hash, ..receipt })
    }

    #[test]
    fn test_validate_sequential_valid_chain() {
        let r1 = create_test_receipt(
            "00000000-0000-0000-0000-000000000001",
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        )
        .unwrap();

        let r2 = create_test_receipt(
            "00000000-0000-0000-0000-000000000002",
            "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
            "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        )
        .unwrap();

        let chain = vec![r1, r2];
        assert!(validate_sequential(&chain).is_ok());
    }

    #[test]
    fn test_validate_sequential_mismatch() {
        let r1 = create_test_receipt(
            "00000000-0000-0000-0000-000000000001",
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        )
        .unwrap();

        let r2 = create_test_receipt(
            "00000000-0000-0000-0000-000000000002",
            "sha256:9999999999999999999999999999999999999999999999999999999999999999".to_string(),
            "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        )
        .unwrap();

        let chain = vec![r1, r2];
        let result = validate_sequential(&chain);
        assert!(result.is_err());
        match result.unwrap_err() {
            CompositionError::SequentialMismatch { receipt_index, .. } => {
                assert_eq!(receipt_index, 1);
            }
            _ => panic!("Expected SequentialMismatch error"),
        }
    }

    #[test]
    fn test_validate_sequential_empty_or_single() {
        let r1 = create_test_receipt(
            "00000000-0000-0000-0000-000000000001",
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        )
        .unwrap();

        assert!(validate_sequential(&[]).is_ok());
        assert!(validate_sequential(&[r1.clone()]).is_ok());
    }

    #[test]
    fn test_compute_tensor_root() {
        let h1 =
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string();
        let h2 =
            "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string();
        let h3 =
            "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string();

        // Order-independent: same hashes in different order produce same root
        let root1 = compute_tensor_root(&[h1.clone(), h2.clone(), h3.clone()]);
        let root2 = compute_tensor_root(&[h3.clone(), h1.clone(), h2.clone()]);
        assert_eq!(root1, root2);

        // Different hashes produce different root
        let root3 = compute_tensor_root(&[h1.clone(), h2.clone()]);
        assert_ne!(root1, root3);
    }

    #[test]
    fn test_validate_link() {
        let _parent_rid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let child_rid = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

        let parent = create_test_receipt(
            "00000000-0000-0000-0000-000000000001",
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        )
        .unwrap();

        let mut parent = Receipt {
            links: vec![child_rid],
            ..parent
        };
        parent.hash = parent.compute_hash().unwrap();

        let child = create_test_receipt(
            "00000000-0000-0000-0000-000000000002",
            "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
            "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        )
        .unwrap();

        assert!(validate_link(&parent, &child).is_ok());
    }

    #[test]
    fn test_validate_link_missing_child() {
        let _parent_rid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let _child_rid = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let other_rid = Uuid::parse_str("00000000-0000-0000-0000-000000000099").unwrap();

        let parent = create_test_receipt(
            "00000000-0000-0000-0000-000000000001",
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        )
        .unwrap();

        let mut parent = Receipt {
            links: vec![other_rid], // Wrong child ID
            ..parent
        };
        parent.hash = parent.compute_hash().unwrap();

        let child = create_test_receipt(
            "00000000-0000-0000-0000-000000000002",
            "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
            "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        )
        .unwrap();

        assert!(validate_link(&parent, &child).is_err());
    }

    #[test]
    fn test_validate_link_domain_mismatch() {
        let _parent_rid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let _child_rid = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

        let parent = create_test_receipt(
            "00000000-0000-0000-0000-000000000001",
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            "sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        )
        .unwrap();

        let mut parent = Receipt {
            links: vec![_child_rid],
            ..parent
        };
        parent.hash = parent.compute_hash().unwrap();

        let child = create_test_receipt(
            "00000000-0000-0000-0000-000000000002",
            "sha256:9999999999999999999999999999999999999999999999999999999999999999".to_string(),
            "sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        )
        .unwrap();

        assert!(validate_link(&parent, &child).is_err());
    }

    #[test]
    fn test_create_identity_receipt() {
        let rid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let shape_hash =
            "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string();

        let ctx = Context {
            policy_ref: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            nonce: None,
            determinism: None,
            identity_ref: None,
        };

        let payload = Payload::DataShape(DataShapePayload {
            schema_hash: shape_hash.clone(),
            sketch_hash: None,
        });

        let receipt = create_identity_receipt(
            rid,
            "0.3.0".to_string(),
            ReceiptKind::DataShape,
            shape_hash.clone(),
            ctx,
            payload,
        )
        .unwrap();

        assert_eq!(receipt.dom, receipt.cod);
        assert_eq!(receipt.dom, shape_hash);
        assert!(receipt.validate_fast().is_ok());
    }
}

//! Execution receipt builder utilities.
//!
//! This module provides utilities for constructing execution receipts,
//! aggregating span commitments, and computing execution roots.

use northroot_receipts::{
    Context, ExecutionPayload, MethodRef, Payload, Receipt, ReceiptKind, ValidationError,
};
use uuid::Uuid;

use crate::execution::state::{compute_execution_roots, generate_trace_id, validate_method_ref};
use crate::commitments::sha256_prefixed;

/// Builder for execution receipts.
///
/// This builder provides a convenient way to construct execution receipts
/// with proper root computation and validation.
#[derive(Debug, Clone)]
pub struct ExecutionReceiptBuilder {
    trace_id: Option<String>,
    method_ref: Option<MethodRef>,
    data_shape_hash: Option<String>,
    span_commitments: Vec<String>,
    identity_root: Option<String>,
}

impl ExecutionReceiptBuilder {
    /// Create a new execution receipt builder.
    pub fn new() -> Self {
        Self {
            trace_id: None,
            method_ref: None,
            data_shape_hash: None,
            span_commitments: Vec::new(),
            identity_root: None,
        }
    }

    /// Set the trace ID.
    pub fn trace_id(mut self, trace_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    /// Generate a trace ID from a seed.
    pub fn trace_id_from_seed(mut self, seed: &str) -> Self {
        self.trace_id = Some(generate_trace_id(Some(seed)));
        self
    }

    /// Set the method reference.
    pub fn method_ref(mut self, method_ref: MethodRef) -> Self {
        self.method_ref = Some(method_ref);
        self
    }

    /// Set the data shape hash.
    pub fn data_shape_hash(mut self, data_shape_hash: String) -> Self {
        self.data_shape_hash = Some(data_shape_hash);
        self
    }

    /// Add a span commitment.
    pub fn add_span_commitment(mut self, commitment: String) -> Self {
        self.span_commitments.push(commitment);
        self
    }

    /// Add multiple span commitments.
    pub fn add_span_commitments(mut self, commitments: Vec<String>) -> Self {
        self.span_commitments.extend(commitments);
        self
    }

    /// Set the identity root.
    pub fn identity_root(mut self, identity_root: String) -> Self {
        self.identity_root = Some(identity_root);
        self
    }

    /// Build the execution receipt.
    ///
    /// # Returns
    ///
    /// Execution receipt with computed roots, or `ValidationError` if validation fails
    pub fn build(
        self,
        rid: Uuid,
        version: String,
        ctx: Context,
    ) -> Result<Receipt, ValidationError> {
        let trace_id = self.trace_id.unwrap_or_else(|| generate_trace_id(None));
        let method_ref = self
            .method_ref
            .ok_or_else(|| ValidationError::MissingField("method_ref".to_string()))?;
        let data_shape_hash = self
            .data_shape_hash
            .ok_or_else(|| ValidationError::MissingField("data_shape_hash".to_string()))?;

        if self.span_commitments.is_empty() {
            return Err(ValidationError::InvalidValue(
                "span_commitments cannot be empty".to_string(),
            ));
        }

        // Validate method reference
        validate_method_ref(&method_ref)?;

        // Compute identity root (use provided or default empty)
        let identity_root = self.identity_root.unwrap_or_else(|| {
            // Default empty identity root
            sha256_prefixed(&[0x00u8])
        });

        // Compute execution roots
        let roots = compute_execution_roots(&self.span_commitments, identity_root);
        let cod = roots.trace_set_root.clone();

        let payload = Payload::Execution(ExecutionPayload {
            trace_id,
            method_ref,
            data_shape_hash: data_shape_hash.clone(),
            span_commitments: self.span_commitments,
            roots,
        });
        let receipt = Receipt {
            rid,
            version,
            kind: ReceiptKind::Execution,
            dom: data_shape_hash,
            cod,
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
}

impl Default for ExecutionReceiptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_receipts::{Context, MethodRef};

    fn create_test_context() -> Context {
        Context {
            policy_ref: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            nonce: None,
            determinism: None,
            identity_ref: None,
        }
    }

    fn create_test_method_ref() -> MethodRef {
        MethodRef {
            method_id: "com.acme/test".to_string(),
            version: "1.0.0".to_string(),
            method_shape_root: "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        }
    }

    #[test]
    fn test_execution_receipt_builder() {
        let rid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let ctx = create_test_context();
        let method_ref = create_test_method_ref();

        let receipt = ExecutionReceiptBuilder::new()
            .trace_id("test-trace".to_string())
            .method_ref(method_ref)
            .data_shape_hash("sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string())
            .add_span_commitment("sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string())
            .build(rid, "0.3.0".to_string(), ctx)
            .unwrap();

        assert_eq!(receipt.kind, ReceiptKind::Execution);
        assert!(receipt.validate_fast().is_ok());
    }

    #[test]
    fn test_execution_receipt_builder_missing_fields() {
        let rid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let ctx = create_test_context();

        // Missing method_ref
        let result = ExecutionReceiptBuilder::new()
            .trace_id("test-trace".to_string())
            .data_shape_hash("sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string())
            .add_span_commitment("sha256:3333333333333333333333333333333333333333333333333333333333333333".to_string())
            .build(rid, "0.3.0".to_string(), ctx.clone());

        assert!(result.is_err());

        // Missing span commitments
        let method_ref = create_test_method_ref();
        let result = ExecutionReceiptBuilder::new()
            .trace_id("test-trace".to_string())
            .method_ref(method_ref)
            .data_shape_hash("sha256:2222222222222222222222222222222222222222222222222222222222222222".to_string())
            .build(rid, "0.3.0".to_string(), ctx);

        assert!(result.is_err());
    }
}


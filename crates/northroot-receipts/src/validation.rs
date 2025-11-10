//! Validation functions for receipts.
//!
//! **Boundary**: This module provides **syntactic validation** (structure, format, schema).
//! For **semantic validation** (policy compliance, business rules), see `northroot-policy`.
//!
//! ## Validation Layers
//!
//! 1. **Syntactic (this crate)**: Format checks, schema validation, structure integrity
//!    - Hash format validation
//!    - Field format validation (timestamps, UUIDs, policy_ref format)
//!    - Kind-specific payload structure
//!    - Composition chain integrity (cod/dom matching)
//!
//! 2. **Semantic (northroot-policy)**: Policy compliance, business rules, constraints
//!    - Policy reference validation (detailed errors)
//!    - Determinism class enforcement
//!    - Tool/region constraint checking
//!    - Policy registry lookups
//!
//! **Dependency rule**: `receipts` must NOT depend on `policy` (see ADR_PLAYBOOK.md).
//! Receipts validation is format-only; policy validation requires policy crate.

use crate::canonical::{compute_hash, validate_hash_format};
use crate::error::ValidationError;
use crate::Receipt;

/// Validate RFC3339 timestamp format.
///
/// Accepts timestamps with optional milliseconds and timezone.
fn validate_rfc3339_timestamp(timestamp: &str) -> bool {
    // Basic RFC3339 format: YYYY-MM-DDTHH:MM:SS[.SSS]Z or with timezone offset
    // We use a simple check - full RFC3339 parsing would require a date library
    // This checks for the basic structure
    timestamp.len() >= 20 && // Minimum: "2025-01-01T00:00:00Z"
        timestamp.chars().nth(4) == Some('-') &&
        timestamp.chars().nth(7) == Some('-') &&
        timestamp.chars().nth(10) == Some('T') &&
        timestamp.chars().nth(13) == Some(':') &&
        timestamp.chars().nth(16) == Some(':') &&
        (timestamp.ends_with('Z') || timestamp.contains('+') || timestamp.contains('-'))
}

/// Validate ISO-4217 currency code (3 uppercase letters).
fn validate_currency_code(currency: &str) -> bool {
    currency.len() == 3 && currency.chars().all(|c| c.is_ascii_uppercase())
}

/// Validate semver-like version format.
///
/// Accepts versions like "0.3.0", "1.0.0", "2.1.3-alpha.1", etc.
fn validate_version_format(version: &str) -> bool {
    // Basic semver check: major.minor.patch[-prerelease][+build]
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return false;
    }
    // Check that major and minor are numeric
    parts[0].parse::<u64>().is_ok() && parts[1].parse::<u64>().is_ok()
}

// UUIDs are validated by the Uuid type itself, so no separate validation needed

/// Validate policy reference format (syntactic check only).
///
/// **Boundary note**: This is a simple format check for receipt structure validation.
/// For detailed policy validation with error messages, use `northroot_policy::validate_policy_ref_format()`.
///
/// Accepts two formats:
/// - Strict: `pol:<namespace>/<name>@<semver>` (e.g., "pol:finops/cost-guard@1.2.0")
/// - Legacy: `pol:<name>-<version>` (e.g., "pol:standard-v1") for backward compatibility
///
/// This function is used during receipt payload validation to ensure the policy_ref field
/// has a valid format. For policy compliance checking, use the policy crate.
fn validate_policy_ref(policy_ref: &str) -> bool {
    if !policy_ref.starts_with("pol:") {
        return false;
    }
    let rest = &policy_ref[4..];

    // Try strict format first: pol:<namespace>/<name>@<semver>
    if let Some(slash_pos) = rest.find('/') {
        let namespace = &rest[..slash_pos];
        let after_slash = &rest[slash_pos + 1..];
        if let Some(at_pos) = after_slash.find('@') {
            let name = &after_slash[..at_pos];
            let version = &after_slash[at_pos + 1..];
            // Validate namespace and name (alphanumeric, dots, underscores, hyphens)
            return namespace
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
                && validate_version_format(version);
        }
    }

    // Legacy format: pol:<name>-<version> (e.g., "pol:standard-v1")
    // Allow alphanumeric, dots, underscores, hyphens
    !rest.is_empty()
        && rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

/// Validate DID URI format: `did:<method>:<method-specific-id>`.
///
/// Examples: "did:key:z6Mk...", "did:web:northroot.dev"
fn validate_did_uri(did: &str) -> bool {
    // Loose validation: ^did:[a-z0-9]+:[A-Za-z0-9.\-_:]+$
    if !did.starts_with("did:") {
        return false;
    }
    let rest = &did[4..];
    if let Some(colon_pos) = rest.find(':') {
        let method = &rest[..colon_pos];
        let method_id = &rest[colon_pos + 1..];
        // Method: lowercase alphanumeric
        !method.is_empty() &&
        method.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) &&
        // Method ID: alphanumeric, dots, hyphens, underscores, colons
        !method_id.is_empty() &&
        method_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' || c == ':')
    } else {
        false
    }
}

/// Validate hash integrity: verify that `receipt.hash == compute_hash(receipt)`.
pub fn validate_hash(receipt: &Receipt) -> Result<(), ValidationError> {
    // Validate hash format
    if !validate_hash_format(&receipt.hash) {
        return Err(ValidationError::InvalidHashFormat(receipt.hash.clone()));
    }

    // Compute hash from canonical body
    let computed =
        compute_hash(receipt).map_err(|e| ValidationError::SerializationError(e.to_string()))?;

    if computed != receipt.hash {
        return Err(ValidationError::HashMismatch {
            expected: receipt.hash.clone(),
            computed,
        });
    }

    Ok(())
}

/// Validate hash format for a string, returning a `Result`.
///
/// Returns `Ok(())` if the hash format is valid, or `InvalidHashFormat` error if invalid.
pub fn validate_hash_format_str(s: &str) -> Result<(), ValidationError> {
    if !validate_hash_format(s) {
        Err(ValidationError::InvalidHashFormat(s.to_string()))
    } else {
        Ok(())
    }
}

/// Validate all hash fields in a receipt payload.
pub fn validate_payload_hashes(receipt: &Receipt) -> Result<(), ValidationError> {
    match &receipt.payload {
        crate::Payload::DataShape(payload) => {
            validate_hash_format_str(&payload.schema_hash)?;
            if let Some(ref sketch_hash) = payload.sketch_hash {
                validate_hash_format_str(sketch_hash)?;
            }
        }
        crate::Payload::MethodShape(payload) => {
            validate_hash_format_str(&payload.root_multiset)?;
            for node in &payload.nodes {
                validate_hash_format_str(&node.span_shape_hash)?;
            }
            if let Some(ref dag_hash) = payload.dag_hash {
                validate_hash_format_str(dag_hash)?;
            }
        }
        crate::Payload::ReasoningShape(payload) => {
            validate_hash_format_str(&payload.intent_hash)?;
            validate_hash_format_str(&payload.dag_hash)?;
            if let Some(ref quality) = payload.quality {
                if let Some(ref review_hash) = quality.review_hash {
                    validate_hash_format_str(review_hash)?;
                }
            }
        }
        crate::Payload::Execution(payload) => {
            validate_hash_format_str(&payload.data_shape_hash)?;
            validate_hash_format_str(&payload.method_ref.method_shape_root)?;
            for span_commitment in &payload.span_commitments {
                validate_hash_format_str(span_commitment)?;
            }
            validate_hash_format_str(&payload.roots.trace_set_root)?;
            validate_hash_format_str(&payload.roots.identity_root)?;
            if let Some(ref trace_seq_root) = payload.roots.trace_seq_root {
                validate_hash_format_str(trace_seq_root)?;
            }
            // Validate new optional fields
            if let Some(ref pac) = payload.pac {
                if pac.len() != 32 {
                    return Err(ValidationError::InvalidValue(format!(
                        "PAC key must be exactly 32 bytes, got {} bytes",
                        pac.len()
                    )));
                }
            }
            if let Some(ref change_epoch) = payload.change_epoch {
                if change_epoch.is_empty() {
                    return Err(ValidationError::InvalidValue(
                        "change_epoch must be non-empty if present".to_string(),
                    ));
                }
            }
            if let Some(hll_cardinality) = payload.hll_cardinality {
                if hll_cardinality == 0 {
                    return Err(ValidationError::InvalidValue(
                        "hll_cardinality must be > 0 if present".to_string(),
                    ));
                }
            }
            if let Some(ref chunk_manifest_hash) = payload.chunk_manifest_hash {
                if chunk_manifest_hash.len() != 32 {
                    return Err(ValidationError::InvalidValue(format!(
                        "chunk_manifest_hash must be exactly 32 bytes, got {} bytes",
                        chunk_manifest_hash.len()
                    )));
                }
            }
            if let Some(ref merkle_root) = payload.merkle_root {
                if merkle_root.len() != 32 {
                    return Err(ValidationError::InvalidValue(format!(
                        "merkle_root must be exactly 32 bytes, got {} bytes",
                        merkle_root.len()
                    )));
                }
            }
        }
        crate::Payload::Spend(payload) => {
            // Validate currency code format
            if !validate_currency_code(&payload.currency) {
                return Err(ValidationError::InvalidValue(format!(
                    "Invalid currency code: {} (expected ISO-4217, 3 uppercase letters)",
                    payload.currency
                )));
            }

            // Validate total_value is approximately dot(meter, unit_prices)
            let mut computed_total = 0.0;
            if let Some(vcpu) = payload.meter.vcpu_sec {
                if let Some(price) = payload.unit_prices.vcpu_sec {
                    computed_total += vcpu * price;
                }
            }
            if let Some(gpu) = payload.meter.gpu_sec {
                if let Some(price) = payload.unit_prices.gpu_sec {
                    computed_total += gpu * price;
                }
            }
            if let Some(gb) = payload.meter.gb_sec {
                if let Some(price) = payload.unit_prices.gb_sec {
                    computed_total += gb * price;
                }
            }
            if let Some(req) = payload.meter.requests {
                if let Some(price) = payload.unit_prices.requests {
                    computed_total += req * price;
                }
            }
            if let Some(energy) = payload.meter.energy_kwh {
                if let Some(price) = payload.unit_prices.energy_kwh {
                    computed_total += energy * price;
                }
            }

            // Allow small floating point differences (ε = 0.0001)
            let epsilon = 0.0001;
            if (computed_total - payload.total_value).abs() > epsilon {
                return Err(ValidationError::InvalidValue(format!(
                    "total_value mismatch: expected ~{}, got {}",
                    computed_total, payload.total_value
                )));
            }

            // Validate layer field if present
            if let Some(ref justification) = payload.justification {
                if let Some(ref layer) = justification.layer {
                    let valid_layers = ["data", "method", "reasoning", "execution"];
                    if !valid_layers.contains(&layer.as_str()) {
                        return Err(ValidationError::InvalidValue(format!(
                            "Invalid layer value: {} (must be one of: {:?})",
                            layer, valid_layers
                        )));
                    }
                }
            }
        }
        crate::Payload::Settlement(_) => {
            // No hash fields to validate in settlement
        }
    }

    // Validate dom and cod hash formats
    validate_hash_format_str(&receipt.dom)?;
    validate_hash_format_str(&receipt.cod)?;

    Ok(())
}

/// Validate receipt payload by kind-specific rules.
pub fn validate_payload(receipt: &Receipt) -> Result<(), ValidationError> {
    // Validate hash formats in payload
    validate_payload_hashes(receipt)?;

    // Validate envelope format fields
    if !validate_version_format(&receipt.version) {
        return Err(ValidationError::InvalidValue(format!(
            "Invalid version format: {} (expected semver-like format)",
            receipt.version
        )));
    }

    // Validate context fields
    if !validate_rfc3339_timestamp(&receipt.ctx.timestamp) {
        return Err(ValidationError::InvalidValue(format!(
            "Invalid timestamp format: {} (expected RFC3339)",
            receipt.ctx.timestamp
        )));
    }

    if let Some(ref policy_ref) = receipt.ctx.policy_ref {
        if !validate_policy_ref(policy_ref) {
            return Err(ValidationError::InvalidValue(format!(
                "Invalid policy_ref format: {} (expected pol:<namespace>/<name>@<semver>)",
                policy_ref
            )));
        }
    }

    if let Some(ref identity_ref) = receipt.ctx.identity_ref {
        if !validate_did_uri(identity_ref) {
            return Err(ValidationError::InvalidValue(format!(
                "Invalid identity_ref format: {} (expected DID URI)",
                identity_ref
            )));
        }
    }

    // UUIDs in links are already validated by the Uuid type during deserialization

    // Kind-specific validation
    match &receipt.payload {
        crate::Payload::Execution(payload) => {
            if payload.span_commitments.is_empty() {
                return Err(ValidationError::InvalidValue(
                    "execution.span_commitments must not be empty".to_string(),
                ));
            }
            // Additional validation for new fields is done in validate_payload_hashes
        }
        crate::Payload::MethodShape(payload) => {
            if payload.nodes.is_empty() {
                return Err(ValidationError::InvalidValue(
                    "method_shape.nodes must not be empty".to_string(),
                ));
            }
        }
        crate::Payload::ReasoningShape(payload) => {
            if payload.node_refs.is_empty() {
                return Err(ValidationError::InvalidValue(
                    "reasoning_shape.node_refs must not be empty".to_string(),
                ));
            }
        }
        crate::Payload::Settlement(payload) => {
            if payload.wur_refs.is_empty() {
                return Err(ValidationError::InvalidValue(
                    "settlement.wur_refs must not be empty".to_string(),
                ));
            }
        }
        _ => {}
    }

    Ok(())
}

/// Validate sequential composition: `cod(R_i) == dom(R_{i+1})`.
pub fn validate_composition(chain: &[Receipt]) -> Result<(), ValidationError> {
    for i in 0..chain.len().saturating_sub(1) {
        let current_cod = &chain[i].cod;
        let next_dom = &chain[i + 1].dom;

        if current_cod != next_dom {
            return Err(ValidationError::CompositionError {
                receipt_index: i + 1,
                expected_cod: current_cod.clone(),
                actual_dom: next_dom.clone(),
            });
        }
    }

    Ok(())
}

/// Full validation of a receipt: hash integrity, payload rules, and format checks.
pub fn validate(receipt: &Receipt) -> Result<(), ValidationError> {
    validate_hash(receipt)?;
    validate_payload(receipt)?;
    Ok(())
}

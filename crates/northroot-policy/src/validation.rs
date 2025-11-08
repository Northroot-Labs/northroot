//! Policy validation for receipts.
//!
//! **Boundary**: This module provides **semantic validation** (policy compliance, business rules).
//! For **syntactic validation** (format, structure, schema), see `northroot-receipts::validation`.
//!
//! ## Validation Layers
//!
//! 1. **Syntactic (northroot-receipts)**: Format checks, schema validation, structure integrity
//!    - Hash format validation
//!    - Field format validation (timestamps, UUIDs, basic policy_ref format)
//!    - Kind-specific payload structure
//!    - Composition chain integrity (cod/dom matching)
//!
//! 2. **Semantic (this module)**: Policy compliance, business rules, constraints
//!    - Policy reference validation (detailed errors, authoritative)
//!    - Determinism class enforcement
//!    - Tool/region constraint checking
//!    - Policy registry lookups
//!
//! **Dependency rule**: `policy` depends on `receipts` but NOT on `engine` (see ADR_PLAYBOOK.md).
//! Policy validation answers "is this allowed?" not "how do I compute this?"

use northroot_receipts::{Context, DeterminismClass, Receipt};

/// Error types for policy validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    /// Policy reference format is invalid
    InvalidPolicyRef {
        /// Invalid policy reference
        policy_ref: String,
        /// Reason for invalidity
        reason: String,
    },
    /// Policy not found in registry
    PolicyNotFound {
        /// Policy reference that was not found
        policy_ref: String,
    },
    /// Determinism class violation
    DeterminismViolation {
        /// Required determinism class
        required: DeterminismClass,
        /// Actual determinism class
        actual: Option<DeterminismClass>,
    },
    /// Tool constraint violation
    ToolViolation {
        /// Tool that violated constraints
        tool: String,
        /// Reason
        reason: String,
    },
    /// Region constraint violation
    RegionViolation {
        /// Region that violated constraints
        region: String,
        /// Reason
        reason: String,
    },
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyError::InvalidPolicyRef { policy_ref, reason } => {
                write!(f, "Invalid policy reference {}: {}", policy_ref, reason)
            }
            PolicyError::PolicyNotFound { policy_ref } => {
                write!(f, "Policy not found: {}", policy_ref)
            }
            PolicyError::DeterminismViolation { required, actual } => {
                write!(
                    f,
                    "Determinism violation: required {:?}, got {:?}",
                    required, actual
                )
            }
            PolicyError::ToolViolation { tool, reason } => {
                write!(f, "Tool violation for {}: {}", tool, reason)
            }
            PolicyError::RegionViolation { region, reason } => {
                write!(f, "Region violation for {}: {}", region, reason)
            }
        }
    }
}

impl std::error::Error for PolicyError {}

/// Validate policy reference format (authoritative, detailed validation).
///
/// **Boundary note**: This is the authoritative policy reference validator with detailed errors.
/// The `northroot-receipts` crate has a simpler format check for syntactic validation only.
///
/// Accepts two formats:
/// - Strict: `pol:<namespace>/<name>@<semver>` (e.g., "pol:finops/cost-guard@1.2.0")
/// - Legacy: `pol:<name>-<version>` (e.g., "pol:standard-v1") for backward compatibility
///
/// # Arguments
///
/// * `policy_ref` - Policy reference string
///
/// # Returns
///
/// `Ok(())` if the format is valid, or `PolicyError` with detailed reason if invalid.
///
/// # Usage
///
/// Use this function for policy compliance checking. For simple format checks during
/// receipt structure validation, `northroot-receipts` uses a simpler boolean check
/// to avoid circular dependencies.
pub fn validate_policy_ref_format(policy_ref: &str) -> Result<(), PolicyError> {
    if !policy_ref.starts_with("pol:") {
        return Err(PolicyError::InvalidPolicyRef {
            policy_ref: policy_ref.to_string(),
            reason: "Policy reference must start with 'pol:'".to_string(),
        });
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
            // Namespace and name must not be empty
            if namespace.is_empty() {
                return Err(PolicyError::InvalidPolicyRef {
                    policy_ref: policy_ref.to_string(),
                    reason: "Namespace cannot be empty".to_string(),
                });
            }

            if !namespace
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
            {
                return Err(PolicyError::InvalidPolicyRef {
                    policy_ref: policy_ref.to_string(),
                    reason: "Invalid namespace format".to_string(),
                });
            }

            if name.is_empty() {
                return Err(PolicyError::InvalidPolicyRef {
                    policy_ref: policy_ref.to_string(),
                    reason: "Name cannot be empty".to_string(),
                });
            }

            if !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
            {
                return Err(PolicyError::InvalidPolicyRef {
                    policy_ref: policy_ref.to_string(),
                    reason: "Invalid name format".to_string(),
                });
            }

            // Basic semver check
            let parts: Vec<&str> = version.split('.').collect();
            if parts.len() < 2 || parts.len() > 3 {
                return Err(PolicyError::InvalidPolicyRef {
                    policy_ref: policy_ref.to_string(),
                    reason: "Invalid version format (expected semver)".to_string(),
                });
            }

            if parts[0].parse::<u64>().is_err() || parts[1].parse::<u64>().is_err() {
                return Err(PolicyError::InvalidPolicyRef {
                    policy_ref: policy_ref.to_string(),
                    reason: "Version must have numeric major and minor".to_string(),
                });
            }

            return Ok(());
        }
    }

    // Legacy format: pol:<name>-<version> (e.g., "pol:standard-v1")
    if rest.is_empty() {
        return Err(PolicyError::InvalidPolicyRef {
            policy_ref: policy_ref.to_string(),
            reason: "Policy reference cannot be empty after 'pol:'".to_string(),
        });
    }

    if !rest
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        return Err(PolicyError::InvalidPolicyRef {
            policy_ref: policy_ref.to_string(),
            reason: "Invalid legacy format: must contain only alphanumeric, dots, underscores, or hyphens".to_string(),
        });
    }

    Ok(())
}

/// Load policy from registry (stub implementation).
///
/// This is a placeholder for future policy registry implementation.
/// Currently returns an error indicating the policy registry is not yet implemented.
///
/// # Arguments
///
/// * `policy_ref` - Policy reference
///
/// # Returns
///
/// Policy document (currently unimplemented)
pub fn load_policy(_policy_ref: &str) -> Result<serde_json::Value, PolicyError> {
    // TODO: Implement policy registry
    Err(PolicyError::PolicyNotFound {
        policy_ref: _policy_ref.to_string(),
    })
}

/// Validate determinism class against policy requirements.
///
/// Checks that the receipt's determinism class (if specified) meets the policy requirements.
///
/// # Arguments
///
/// * `ctx` - Receipt context
/// * `required` - Required determinism class from policy
///
/// # Returns
///
/// `Ok(())` if determinism class is acceptable, or `PolicyError` if violation.
///
/// # Note
///
/// Determinism classes form a hierarchy:
/// - `Strict` is the most restrictive (bit-identical)
/// - `Bounded` allows some tolerance
/// - `Observational` is the least restrictive (no reproducibility claim)
///
/// A receipt with a more restrictive class than required is acceptable.
pub fn validate_determinism(
    ctx: &Context,
    required: DeterminismClass,
) -> Result<(), PolicyError> {
    let actual = ctx.determinism.as_ref();

    match (required, actual) {
        // If policy requires strict, receipt must be strict
        (DeterminismClass::Strict, Some(DeterminismClass::Strict)) => Ok(()),
        (DeterminismClass::Strict, _) => Err(PolicyError::DeterminismViolation {
            required: DeterminismClass::Strict,
            actual: actual.cloned(),
        }),

        // If policy requires bounded, receipt can be strict or bounded
        (DeterminismClass::Bounded, Some(DeterminismClass::Strict)) => Ok(()),
        (DeterminismClass::Bounded, Some(DeterminismClass::Bounded)) => Ok(()),
        (DeterminismClass::Bounded, _) => Err(PolicyError::DeterminismViolation {
            required: DeterminismClass::Bounded,
            actual: actual.cloned(),
        }),

        // If policy requires observational, any class is acceptable
        (DeterminismClass::Observational, _) => Ok(()),
    }
}

/// Validate receipt against policy constraints (stub).
///
/// This function performs policy-based validation of a receipt. Currently,
/// it validates policy reference format and determinism class. Full policy
/// constraint checking (tools, regions, tolerances) is planned for future versions.
///
/// # Arguments
///
/// * `receipt` - Receipt to validate
/// * `policy_ref` - Optional policy reference (if None, uses receipt's policy_ref)
///
/// # Returns
///
/// `Ok(())` if receipt complies with policy, or `PolicyError` if violation.
pub fn validate_policy(
    receipt: &Receipt,
    policy_ref: Option<&str>,
) -> Result<(), PolicyError> {
    let policy_ref = policy_ref
        .or(receipt.ctx.policy_ref.as_deref())
        .ok_or_else(|| PolicyError::InvalidPolicyRef {
            policy_ref: "none".to_string(),
            reason: "No policy reference provided".to_string(),
        })?;

    // Validate policy reference format
    validate_policy_ref_format(policy_ref)?;

    // Try to load policy (currently stub)
    let _policy = load_policy(policy_ref)?;

    // TODO: Extract determinism requirement from policy
    // For now, we'll use a default if not specified in policy
    // This is a placeholder for future policy parsing

    // TODO: Validate tool constraints
    // TODO: Validate region constraints
    // TODO: Validate tolerance constraints

    Ok(())
}

/// Validate tool constraints (stub).
///
/// This is a placeholder for future tool constraint validation.
///
/// # Arguments
///
/// * `tool` - Tool identifier
/// * `policy` - Policy document
///
/// # Returns
///
/// `Ok(())` if tool is allowed, or `PolicyError` if violation.
pub fn validate_tool_constraints(
    _tool: &str,
    _policy: &serde_json::Value,
) -> Result<(), PolicyError> {
    // TODO: Implement tool constraint checking
    Ok(())
}

/// Validate region constraints (stub).
///
/// This is a placeholder for future region constraint validation.
///
/// # Arguments
///
/// * `region` - Region identifier
/// * `policy` - Policy document
///
/// # Returns
///
/// `Ok(())` if region is allowed, or `PolicyError` if violation.
pub fn validate_region_constraints(
    _region: &str,
    _policy: &serde_json::Value,
) -> Result<(), PolicyError> {
    // TODO: Implement region constraint checking
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_receipts::{Context, DeterminismClass};
    use uuid::Uuid;

    #[test]
    fn test_validate_policy_ref_format_strict() {
        assert!(validate_policy_ref_format("pol:finops/cost-guard@1.2.0").is_ok());
        assert!(validate_policy_ref_format("pol:namespace/name@2.0.1").is_ok());
    }

    #[test]
    fn test_validate_policy_ref_format_legacy() {
        assert!(validate_policy_ref_format("pol:standard-v1").is_ok());
        assert!(validate_policy_ref_format("pol:my-policy-v2").is_ok());
    }

    #[test]
    fn test_validate_policy_ref_format_invalid() {
        assert!(validate_policy_ref_format("invalid").is_err());
        assert!(validate_policy_ref_format("pol:").is_err());
        assert!(validate_policy_ref_format("pol:namespace/name").is_err()); // Missing @version
    }

    #[test]
    fn test_validate_determinism_strict_required() {
        let ctx_strict = Context {
            policy_ref: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            nonce: None,
            determinism: Some(DeterminismClass::Strict),
            identity_ref: None,
        };

        let ctx_bounded = Context {
            determinism: Some(DeterminismClass::Bounded),
            ..ctx_strict.clone()
        };

        assert!(validate_determinism(&ctx_strict, DeterminismClass::Strict).is_ok());
        assert!(validate_determinism(&ctx_bounded, DeterminismClass::Strict).is_err());
    }

    #[test]
    fn test_validate_determinism_bounded_required() {
        let ctx_strict = Context {
            policy_ref: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            nonce: None,
            determinism: Some(DeterminismClass::Strict),
            identity_ref: None,
        };

        let ctx_bounded = Context {
            determinism: Some(DeterminismClass::Bounded),
            ..ctx_strict.clone()
        };

        assert!(validate_determinism(&ctx_strict, DeterminismClass::Bounded).is_ok());
        assert!(validate_determinism(&ctx_bounded, DeterminismClass::Bounded).is_ok());
    }

    #[test]
    fn test_validate_determinism_observational_required() {
        let ctx_strict = Context {
            policy_ref: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            nonce: None,
            determinism: Some(DeterminismClass::Strict),
            identity_ref: None,
        };

        let ctx_bounded = Context {
            determinism: Some(DeterminismClass::Bounded),
            ..ctx_strict.clone()
        };

        let ctx_observational = Context {
            determinism: Some(DeterminismClass::Observational),
            ..ctx_strict.clone()
        };

        assert!(validate_determinism(&ctx_strict, DeterminismClass::Observational).is_ok());
        assert!(validate_determinism(&ctx_bounded, DeterminismClass::Observational).is_ok());
        assert!(validate_determinism(&ctx_observational, DeterminismClass::Observational).is_ok());
    }
}


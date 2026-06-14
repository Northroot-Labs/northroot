//! ClearlyOps ops-core profile over Northroot records.
//!
//! This crate is intentionally above core. It owns ClearlyOps and ClientOps
//! vocabulary such as custody classes, domain types, and application predicates
//! without changing the Core V0 record shape.

#![deny(missing_docs)]

use northroot_record::{validate_record, Record};

/// Seed evaluation completed predicate.
pub const AG_SEED_EVAL_COMPLETED: &str = "ag.seed_eval.completed";
/// Client export prepared predicate.
pub const CLIENT_EXPORT_PREPARED: &str = "clientops.export.prepared";
/// Delivery artifact classified predicate.
pub const DELIVERY_ARTIFACT_CLASSIFIED: &str = "clearlyops.delivery_artifact.classified";

/// Supported ClearlyOps custody classes.
pub const CUSTODY_CLASSES: &[&str] = &["public", "internal", "client_sensitive", "regulated"];
/// Supported ClearlyOps resource domain types.
pub const DOMAIN_TYPES: &[&str] = &["crop_plan", "seed_eval", "delivery_artifact"];

/// Validates a ClearlyOps profile record.
///
/// # Errors
///
/// Returns [`ClearlyOpsError`] when core validation or profile constraints fail.
pub fn validate_clearlyops_record(record: &Record) -> Result<(), ClearlyOpsError> {
    validate_record(record)?;
    if !is_clearlyops_predicate(&record.statement.predicate) {
        return Err(ClearlyOpsError::UnknownPredicate(
            record.statement.predicate.clone(),
        ));
    }
    let custody_class = record
        .context
        .scope
        .as_ref()
        .map(|scope| scope.custody_class.as_str())
        .ok_or(ClearlyOpsError::MissingField("context.scope.custody_class"))?;
    if !CUSTODY_CLASSES.contains(&custody_class) {
        return Err(ClearlyOpsError::UnknownCustodyClass(
            custody_class.to_string(),
        ));
    }
    if let Some(domain_type) = record
        .payload
        .get("domain_type")
        .and_then(|value| value.as_str())
    {
        if !DOMAIN_TYPES.contains(&domain_type) {
            return Err(ClearlyOpsError::UnknownDomainType(domain_type.to_string()));
        }
    }
    Ok(())
}

/// ClearlyOps profile validation error.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ClearlyOpsError {
    /// Core validation failed.
    #[error("record validation failed: {0}")]
    Record(#[from] northroot_record::ValidationError),
    /// Required field is absent.
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    /// Predicate is not owned by this profile.
    #[error("unknown clearlyops predicate: {0}")]
    UnknownPredicate(String),
    /// Custody class is not supported by this profile.
    #[error("unknown custody class: {0}")]
    UnknownCustodyClass(String),
    /// Domain type is not supported by this profile.
    #[error("unknown domain type: {0}")]
    UnknownDomainType(String),
}

fn is_clearlyops_predicate(predicate: &str) -> bool {
    matches!(
        predicate,
        AG_SEED_EVAL_COMPLETED | CLIENT_EXPORT_PREPARED | DELIVERY_ARTIFACT_CLASSIFIED
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_record::{
        compute_record_id, Context, Record, RecordRefs, RecordRole, Scope, Statement,
    };
    use serde_json::json;

    #[test]
    fn clearlyops_vocabulary_stays_above_core() {
        let mut record = Record::new(
            RecordRole::Event,
            Statement {
                subject: "entity:principal:codex".to_string(),
                predicate: AG_SEED_EVAL_COMPLETED.to_string(),
                object: "resource:seed_eval:apd-2026".to_string(),
            },
            Context {
                node_id: Some("node:apd_croptrak_2026".to_string()),
                time: Some("2026-06-14T18:00:00Z".to_string()),
                scope: Some(Scope {
                    workspace_id: "workspace:clientops-local".to_string(),
                    custody_class: "client_sensitive".to_string(),
                }),
                ..Context::default()
            },
            RecordRefs {
                inputs: vec!["resource:document:seed-report".to_string()],
                outputs: vec!["resource:seed_eval:apd-2026".to_string()],
                ..RecordRefs::default()
            },
            json!({"domain_type": "seed_eval"}),
        );
        record.id = compute_record_id(&record).unwrap();
        validate_clearlyops_record(&record).unwrap();
    }
}

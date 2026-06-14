//! Execution method registry contracts.
//!
//! This layer validates that a record references an advertised execution method
//! kind/ref. It does not invoke tools, spawn processes, call connectors, or
//! decide whether a command is allowed.

#![deny(missing_docs)]

use northroot_record::{validate_record, MethodKind, Record};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Method descriptor advertised by an execution adapter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodDescriptor {
    /// Method kind.
    pub kind: MethodKind,
    /// Stable method reference.
    #[serde(rename = "ref")]
    pub ref_: String,
    /// Human-readable owner or adapter label.
    pub provider: String,
}

/// Registry of advertised execution methods.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodRegistry {
    methods: BTreeMap<String, MethodDescriptor>,
}

impl MethodRegistry {
    /// Registers a method descriptor.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError`] if the descriptor is blank or conflicts.
    pub fn register(&mut self, descriptor: MethodDescriptor) -> Result<(), ExecutionError> {
        require_non_empty("method.ref", &descriptor.ref_)?;
        require_non_empty("method.provider", &descriptor.provider)?;
        if let Some(existing) = self.methods.get(&descriptor.ref_) {
            if existing != &descriptor {
                return Err(ExecutionError::MethodConflict(descriptor.ref_));
            }
        }
        self.methods.insert(descriptor.ref_.clone(), descriptor);
        Ok(())
    }

    /// Returns a method descriptor by reference.
    pub fn get(&self, ref_: &str) -> Option<&MethodDescriptor> {
        self.methods.get(ref_)
    }

    /// Validates that a record's context method is registered.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError`] if the record is invalid or references an
    /// unknown/incompatible method.
    pub fn validate_record_method(&self, record: &Record) -> Result<(), ExecutionError> {
        validate_record(record)?;
        let method = record
            .context
            .method
            .as_ref()
            .ok_or(ExecutionError::MissingMethod)?;
        let descriptor = self
            .methods
            .get(&method.ref_)
            .ok_or_else(|| ExecutionError::UnknownMethod(method.ref_.clone()))?;
        if descriptor.kind != method.kind {
            return Err(ExecutionError::MethodKindMismatch {
                ref_: method.ref_.clone(),
                expected: descriptor.kind.clone(),
                found: method.kind.clone(),
            });
        }
        Ok(())
    }
}

/// Execution registry validation error.
#[derive(thiserror::Error, Debug)]
pub enum ExecutionError {
    /// Core record validation failed.
    #[error("record validation failed: {0}")]
    Record(#[from] northroot_record::ValidationError),
    /// Required field is missing.
    #[error("missing field: {0}")]
    MissingField(&'static str),
    /// Record has no method.
    #[error("record context has no method")]
    MissingMethod,
    /// Method was not registered.
    #[error("unknown method: {0}")]
    UnknownMethod(String),
    /// Method reference is registered with conflicting metadata.
    #[error("method conflict: {0}")]
    MethodConflict(String),
    /// Registered method kind differs from record context.
    #[error("method kind mismatch for {ref_}: expected {expected:?}, found {found:?}")]
    MethodKindMismatch {
        /// Method reference.
        ref_: String,
        /// Registered kind.
        expected: MethodKind,
        /// Record kind.
        found: MethodKind,
    },
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), ExecutionError> {
    if value.trim().is_empty() {
        Err(ExecutionError::MissingField(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use northroot_record::{
        compute_record_id, Context, Method, Record, RecordRefs, RecordRole, Statement,
    };
    use serde_json::json;

    #[test]
    fn registry_validates_advertised_method_without_execution() {
        let mut registry = MethodRegistry::default();
        registry
            .register(MethodDescriptor {
                kind: MethodKind::Tool,
                ref_: "tool:classify-document".to_string(),
                provider: "local-test".to_string(),
            })
            .unwrap();

        let mut record = Record::new(
            RecordRole::Command,
            Statement {
                subject: "entity:principal:codex".to_string(),
                predicate: "resource.classified".to_string(),
                object: "resource:document:seed-report".to_string(),
            },
            Context {
                method: Some(Method {
                    kind: MethodKind::Tool,
                    ref_: "tool:classify-document".to_string(),
                }),
                ..Context::default()
            },
            RecordRefs::default(),
            json!({}),
        );
        record.id = compute_record_id(&record).unwrap();

        registry.validate_record_method(&record).unwrap();
    }
}

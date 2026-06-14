//! Node and workspace blueprint conventions over Northroot records.
//!
//! This layer defines storage and namespace conventions (`journal/`, `vault/`,
//! `state/`, resource/entity namespaces, and custody classes). It does not
//! interpret record predicates or make policy decisions.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// Node manifest schema identifier.
pub const NODE_MANIFEST_SCHEMA_V0: &str = "northroot.node-manifest.v0";
/// Workspace manifest schema identifier.
pub const WORKSPACE_MANIFEST_SCHEMA_V0: &str = "northroot.workspace-manifest.v0";

/// Node-level manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeManifest {
    /// Schema identifier.
    pub schema: String,
    /// Stable node identifier.
    pub node_id: String,
    /// Resource namespace prefix, for example `resource:`.
    pub resource_namespace: String,
    /// Entity namespace prefix, for example `entity:`.
    pub entity_namespace: String,
    /// Relative path to journal storage.
    pub journal_path: String,
    /// Relative path to vault object storage.
    pub vault_path: String,
    /// Relative path to projection/index storage.
    pub state_path: String,
}

/// Workspace-level manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    /// Schema identifier.
    pub schema: String,
    /// Stable workspace identifier.
    pub workspace_id: String,
    /// Owning node identifier.
    pub node_id: String,
    /// Supported custody classes.
    pub custody_classes: Vec<String>,
    /// Relative path to this workspace's journal directory.
    pub journal_path: String,
    /// Relative path to this workspace's vault directory.
    pub vault_path: String,
    /// Relative path to this workspace's state directory.
    pub state_path: String,
}

/// Manifest validation error.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ManifestError {
    /// Schema is not known.
    #[error("unknown manifest schema: {0}")]
    UnknownSchema(String),
    /// Required field is missing or blank.
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    /// Identifier uses an unexpected namespace.
    #[error("namespace mismatch at {field}: expected {expected}, found {found}")]
    NamespaceMismatch {
        /// Field being checked.
        field: &'static str,
        /// Expected prefix.
        expected: &'static str,
        /// Found value.
        found: String,
    },
    /// Storage path is not a boring relative directory.
    #[error("invalid storage path at {field}: {path}")]
    InvalidPath {
        /// Field being checked.
        field: &'static str,
        /// Supplied path.
        path: String,
    },
}

impl NodeManifest {
    /// Validates node blueprint invariants.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError`] if the manifest violates Layer 1 conventions.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.schema != NODE_MANIFEST_SCHEMA_V0 {
            return Err(ManifestError::UnknownSchema(self.schema.clone()));
        }
        require_prefix("node_id", "node:", &self.node_id)?;
        require_non_empty("resource_namespace", &self.resource_namespace)?;
        require_non_empty("entity_namespace", &self.entity_namespace)?;
        require_storage_path("journal_path", &self.journal_path, "journal")?;
        require_storage_path("vault_path", &self.vault_path, "vault")?;
        require_storage_path("state_path", &self.state_path, "state")?;
        Ok(())
    }
}

impl WorkspaceManifest {
    /// Validates workspace blueprint invariants.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError`] if the manifest violates Layer 1 conventions.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.schema != WORKSPACE_MANIFEST_SCHEMA_V0 {
            return Err(ManifestError::UnknownSchema(self.schema.clone()));
        }
        require_prefix("workspace_id", "workspace:", &self.workspace_id)?;
        require_prefix("node_id", "node:", &self.node_id)?;
        if self.custody_classes.is_empty() {
            return Err(ManifestError::MissingField("custody_classes"));
        }
        for custody_class in &self.custody_classes {
            require_non_empty("custody_classes[]", custody_class)?;
        }
        require_storage_path("journal_path", &self.journal_path, "journal")?;
        require_storage_path("vault_path", &self.vault_path, "vault")?;
        require_storage_path("state_path", &self.state_path, "state")?;
        Ok(())
    }
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), ManifestError> {
    if value.trim().is_empty() {
        Err(ManifestError::MissingField(field))
    } else {
        Ok(())
    }
}

fn require_prefix(
    field: &'static str,
    expected: &'static str,
    value: &str,
) -> Result<(), ManifestError> {
    if value.starts_with(expected) && value.len() > expected.len() {
        Ok(())
    } else {
        Err(ManifestError::NamespaceMismatch {
            field,
            expected,
            found: value.to_string(),
        })
    }
}

fn require_storage_path(
    field: &'static str,
    value: &str,
    expected_first_segment: &'static str,
) -> Result<(), ManifestError> {
    require_non_empty(field, value)?;
    let bad = value.starts_with('/')
        || value.contains("..")
        || value.contains('\\')
        || value
            .split('/')
            .next()
            .is_none_or(|segment| segment != expected_first_segment);
    if bad {
        Err(ManifestError::InvalidPath {
            field,
            path: value.to_string(),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_boring_node_and_workspace_layout() {
        let node = NodeManifest {
            schema: NODE_MANIFEST_SCHEMA_V0.to_string(),
            node_id: "node:apd_croptrak_2026".to_string(),
            resource_namespace: "resource:".to_string(),
            entity_namespace: "entity:".to_string(),
            journal_path: "journal/".to_string(),
            vault_path: "vault/".to_string(),
            state_path: "state/".to_string(),
        };
        node.validate().unwrap();

        let workspace = WorkspaceManifest {
            schema: WORKSPACE_MANIFEST_SCHEMA_V0.to_string(),
            workspace_id: "workspace:clientops-local".to_string(),
            node_id: node.node_id,
            custody_classes: vec!["client_sensitive".to_string()],
            journal_path: "journal/clientops-local".to_string(),
            vault_path: "vault/clientops-local".to_string(),
            state_path: "state/clientops-local".to_string(),
        };
        workspace.validate().unwrap();
    }
}

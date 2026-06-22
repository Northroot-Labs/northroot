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

/// Default local metadata index URI for a node root.
pub const DEFAULT_INDEX_URI: &str = "sqlite://state/node.db";
/// Default local object store URI for a node root.
pub const DEFAULT_OBJECT_STORE_URI: &str = "fs://vault/objects";

/// Node-level manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeManifest {
    /// Schema identifier.
    pub schema: String,
    /// Stable node identifier.
    pub node_id: String,
    /// Human-friendly node alias. Slugs are scoped aliases, not durable identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// Custody/execution boundary represented by this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
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
    /// Active metadata index binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<IndexBinding>,
    /// Object store bindings. The first `primary` store is the active default.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub object_stores: Vec<ObjectStoreBinding>,
}

/// Metadata index backend binding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexBinding {
    /// Backend kind, for example `sqlite` or `postgres`.
    pub kind: String,
    /// Backend URI. Local nodes default to `sqlite://state/node.db`.
    pub uri: String,
}

/// Object store backend binding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectStoreBinding {
    /// Stable store alias within this node.
    pub id: String,
    /// Backend kind, for example `fs`, `s3`, `r2`, or `restic`.
    pub kind: String,
    /// Backend URI. Local nodes default to `fs://vault/objects`.
    pub uri: String,
    /// Store role, for example `primary`, `archive`, or `cache`.
    pub role: String,
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
    /// Node scope is unknown.
    #[error("invalid node scope: {0}")]
    InvalidScope(String),
    /// Slug or binding identifier is not portable.
    #[error("invalid slug at {field}: {value}")]
    InvalidSlug {
        /// Field being checked.
        field: &'static str,
        /// Supplied value.
        value: String,
    },
    /// Storage/index backend kind is unknown.
    #[error("invalid backend kind at {field}: {kind}")]
    InvalidBackendKind {
        /// Field being checked.
        field: &'static str,
        /// Supplied kind.
        kind: String,
    },
    /// Storage/index URI is not compatible with its backend kind.
    #[error("invalid backend URI at {field}: {uri}")]
    InvalidBackendUri {
        /// Field being checked.
        field: &'static str,
        /// Supplied URI.
        uri: String,
    },
    /// Object store role is unknown.
    #[error("invalid object store role: {0}")]
    InvalidStoreRole(String),
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
        if let Some(slug) = &self.slug {
            require_slug("slug", slug)?;
        }
        if let Some(scope) = &self.scope {
            require_scope(scope)?;
        }
        require_non_empty("resource_namespace", &self.resource_namespace)?;
        require_non_empty("entity_namespace", &self.entity_namespace)?;
        require_storage_path("journal_path", &self.journal_path, "journal")?;
        require_storage_path("vault_path", &self.vault_path, "vault")?;
        require_storage_path("state_path", &self.state_path, "state")?;
        if let Some(index) = &self.index {
            index.validate()?;
        }
        for store in &self.object_stores {
            store.validate()?;
        }
        Ok(())
    }
}

impl NodeManifest {
    /// Creates a portable local node manifest.
    ///
    /// The caller supplies the unique suffix so initialization remains
    /// deterministic and testable. Slug validation keeps human aliases portable;
    /// the generated `node_id` is the durable identifier.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError`] if the slug or scope is invalid.
    pub fn local(slug: &str, scope: &str, unique_suffix: &str) -> Result<Self, ManifestError> {
        require_slug("slug", slug)?;
        require_scope(scope)?;
        require_slug("unique_suffix", unique_suffix)?;
        let manifest = Self {
            schema: NODE_MANIFEST_SCHEMA_V0.to_string(),
            node_id: format!("node:{slug}-{unique_suffix}"),
            slug: Some(slug.to_string()),
            scope: Some(scope.to_string()),
            resource_namespace: "resource:".to_string(),
            entity_namespace: "entity:".to_string(),
            journal_path: "journal/".to_string(),
            vault_path: "vault/".to_string(),
            state_path: "state/".to_string(),
            index: Some(IndexBinding {
                kind: "sqlite".to_string(),
                uri: DEFAULT_INDEX_URI.to_string(),
            }),
            object_stores: vec![ObjectStoreBinding {
                id: "local".to_string(),
                kind: "fs".to_string(),
                uri: DEFAULT_OBJECT_STORE_URI.to_string(),
                role: "primary".to_string(),
            }],
        };
        manifest.validate()?;
        Ok(manifest)
    }
}

impl IndexBinding {
    /// Validates metadata index backend invariants.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError`] if the backend kind or URI is invalid.
    pub fn validate(&self) -> Result<(), ManifestError> {
        match self.kind.as_str() {
            "sqlite" => require_uri_prefix("index.uri", &self.uri, "sqlite://"),
            "postgres" => require_uri_prefix("index.uri", &self.uri, "postgres://"),
            other => Err(ManifestError::InvalidBackendKind {
                field: "index.kind",
                kind: other.to_string(),
            }),
        }
    }
}

impl ObjectStoreBinding {
    /// Validates object store backend invariants.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError`] if the backend kind, URI, id, or role is invalid.
    pub fn validate(&self) -> Result<(), ManifestError> {
        require_slug("object_stores[].id", &self.id)?;
        match self.kind.as_str() {
            "fs" => require_uri_prefix("object_stores[].uri", &self.uri, "fs://")?,
            "s3" => require_uri_prefix("object_stores[].uri", &self.uri, "s3://")?,
            "r2" => require_uri_prefix("object_stores[].uri", &self.uri, "r2://")?,
            "restic" => require_uri_prefix("object_stores[].uri", &self.uri, "restic://")?,
            other => {
                return Err(ManifestError::InvalidBackendKind {
                    field: "object_stores[].kind",
                    kind: other.to_string(),
                });
            }
        }
        match self.role.as_str() {
            "primary" | "archive" | "cache" => Ok(()),
            other => Err(ManifestError::InvalidStoreRole(other.to_string())),
        }
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

fn require_scope(value: &str) -> Result<(), ManifestError> {
    match value {
        "user-machine" | "org" | "service" | "workspace" => Ok(()),
        other => Err(ManifestError::InvalidScope(other.to_string())),
    }
}

fn require_slug(field: &'static str, value: &str) -> Result<(), ManifestError> {
    if value.is_empty()
        || value.len() > 96
        || value.starts_with('-')
        || value.ends_with('-')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        Err(ManifestError::InvalidSlug {
            field,
            value: value.to_string(),
        })
    } else {
        Ok(())
    }
}

fn require_uri_prefix(
    field: &'static str,
    value: &str,
    expected: &'static str,
) -> Result<(), ManifestError> {
    if value.starts_with(expected) && value.len() > expected.len() {
        Ok(())
    } else {
        Err(ManifestError::InvalidBackendUri {
            field,
            uri: value.to_string(),
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
            node_id: "node:ag_demo_2026".to_string(),
            slug: None,
            scope: None,
            resource_namespace: "resource:".to_string(),
            entity_namespace: "entity:".to_string(),
            journal_path: "journal/".to_string(),
            vault_path: "vault/".to_string(),
            state_path: "state/".to_string(),
            index: None,
            object_stores: Vec::new(),
        };
        node.validate().unwrap();

        let workspace = WorkspaceManifest {
            schema: WORKSPACE_MANIFEST_SCHEMA_V0.to_string(),
            workspace_id: "workspace:ag-demo".to_string(),
            node_id: node.node_id,
            custody_classes: vec!["restricted".to_string()],
            journal_path: "journal/ag-demo".to_string(),
            vault_path: "vault/ag-demo".to_string(),
            state_path: "state/ag-demo".to_string(),
        };
        workspace.validate().unwrap();
    }

    #[test]
    fn creates_local_node_manifest_with_swappable_storage_bindings() {
        let node = NodeManifest::local("steacutter-macbookair", "user-machine", "01abc")
            .expect("local manifest");

        assert_eq!(node.node_id, "node:steacutter-macbookair-01abc");
        assert_eq!(node.slug.as_deref(), Some("steacutter-macbookair"));
        assert_eq!(node.scope.as_deref(), Some("user-machine"));
        assert_eq!(
            node.index.as_ref().map(|index| index.uri.as_str()),
            Some(DEFAULT_INDEX_URI)
        );
        assert_eq!(node.object_stores[0].uri, DEFAULT_OBJECT_STORE_URI);
        node.validate().unwrap();
    }

    #[test]
    fn rejects_absolute_storage_paths_and_nonportable_slugs() {
        let mut node = NodeManifest::local("demo-node", "user-machine", "01abc").unwrap();
        node.journal_path = "/Users/example/.northroot/journal".to_string();
        assert!(matches!(
            node.validate(),
            Err(ManifestError::InvalidPath {
                field: "journal_path",
                ..
            })
        ));

        let err = NodeManifest::local("Demo Node", "user-machine", "01abc").unwrap_err();
        assert!(matches!(
            err,
            ManifestError::InvalidSlug { field: "slug", .. }
        ));
    }

    #[test]
    fn accepts_cloud_object_store_and_postgres_index_contracts() {
        let mut node = NodeManifest::local("org-node", "org", "01abc").unwrap();
        node.index = Some(IndexBinding {
            kind: "postgres".to_string(),
            uri: "postgres://northroot-index".to_string(),
        });
        node.object_stores = vec![ObjectStoreBinding {
            id: "primary-r2".to_string(),
            kind: "r2".to_string(),
            uri: "r2://northroot-node-objects".to_string(),
            role: "primary".to_string(),
        }];

        node.validate().unwrap();
    }
}

//! Workspace and workspace vault manifests.

use crate::object_store::{LocalObjectStore, ObjectStore};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

const WORKSPACE_DIR: &str = ".northroot";
const WORKSPACE_MANIFEST: &str = "workspace.json";
const WORKSPACE_SCHEMA_VERSION: &str = "northroot.workspace.v0";
const CONNECTION_SCHEMA_VERSION: &str = "northroot.connection.v0";

const VAULT_DIRS: &[&str] = &[
    "raw",
    "derived",
    "indexes",
    "logs",
    "receipts",
    "manifests",
    "manifests/connections",
];

/// Workspace operation errors.
#[derive(Debug, Error)]
pub enum WorkspaceError {
    /// Workspace name is invalid.
    #[error("workspace name must not be empty")]
    InvalidName,
    /// Workspace manifest was not found.
    #[error("workspace manifest not found at {0}")]
    MissingManifest(String),
    /// I/O failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization or parsing failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// Object store operation failed.
    #[error("object store error: {0}")]
    ObjectStore(#[from] crate::object_store::ObjectStoreError),
}

/// Result alias for workspace operations.
pub type Result<T> = std::result::Result<T, WorkspaceError>;

/// Workspace manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceManifest {
    /// Manifest schema version.
    pub schema_version: String,
    /// Human-readable workspace name.
    pub name: String,
    /// Workspace root path.
    pub workspace_root: String,
    /// Workspace vault root path.
    pub vault_root: String,
    /// Object store configuration.
    pub object_store: ObjectStoreConfig,
    /// Connection manifest references.
    pub connections: Vec<ConnectionRef>,
    /// Enabled reusable skill package identifiers.
    pub enabled_skills: Vec<String>,
    /// Manifest creation timestamp.
    pub created_at: String,
}

/// Object store configuration.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObjectStoreConfig {
    /// Backend identifier. V0 supports `local`.
    pub backend: String,
    /// Backend root path.
    pub root: String,
}

/// Reference to a provider connection manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConnectionRef {
    /// Provider identifier.
    pub provider: String,
    /// Connection mode, such as `readonly`.
    pub mode: String,
    /// Object-store path to the connection manifest.
    pub manifest_path: String,
}

/// Provider connection manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConnectionManifest {
    /// Manifest schema version.
    pub schema_version: String,
    /// Provider identifier.
    pub provider: String,
    /// Connection mode.
    pub mode: String,
    /// Non-secret auth reference.
    pub auth_ref: String,
    /// Requested/readiness scopes.
    pub scopes: Vec<String>,
    /// Workspace name this connection is bound to.
    pub workspace: String,
    /// Secret posture statement.
    pub secret_posture: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// Workspace status report.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceStatus {
    /// Workspace manifest.
    pub workspace: WorkspaceManifest,
    /// Vault directory readiness.
    pub vault_dirs: Vec<VaultDirStatus>,
    /// Whether the named workspace manifest exists in the vault object store.
    pub vault_manifest_exists: bool,
    /// Number of object-store paths under `manifests/`.
    pub manifest_object_count: usize,
    /// Number of connection manifests.
    pub connection_count: usize,
}

/// Per-directory readiness for a vault path.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VaultDirStatus {
    /// Relative vault directory.
    pub path: String,
    /// Whether the directory exists.
    pub exists: bool,
}

/// Initialize a workspace and local workspace vault.
pub fn init_workspace(name: &str, root: impl AsRef<Path>) -> Result<WorkspaceManifest> {
    if name.trim().is_empty() {
        return Err(WorkspaceError::InvalidName);
    }
    let workspace_root = root.as_ref().to_path_buf();
    fs::create_dir_all(&workspace_root)?;
    let vault_root = workspace_root.join("vault");
    for dir in VAULT_DIRS {
        fs::create_dir_all(vault_root.join(dir))?;
    }
    fs::create_dir_all(workspace_root.join(WORKSPACE_DIR))?;

    let manifest = WorkspaceManifest {
        schema_version: WORKSPACE_SCHEMA_VERSION.to_string(),
        name: name.to_string(),
        workspace_root: workspace_root.display().to_string(),
        vault_root: vault_root.display().to_string(),
        object_store: ObjectStoreConfig {
            backend: "local".to_string(),
            root: vault_root.display().to_string(),
        },
        connections: Vec::new(),
        enabled_skills: Vec::new(),
        created_at: now_utc_string(),
    };
    write_workspace_manifest(&manifest)?;
    Ok(manifest)
}

/// Load a workspace manifest from a workspace root.
pub fn load_workspace(root: impl AsRef<Path>) -> Result<WorkspaceManifest> {
    let path = workspace_manifest_path(root.as_ref());
    if !path.exists() {
        return Err(WorkspaceError::MissingManifest(path.display().to_string()));
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

/// Compute workspace status.
pub fn workspace_status(root: impl AsRef<Path>) -> Result<WorkspaceStatus> {
    let manifest = load_workspace(root)?;
    let vault_root = PathBuf::from(&manifest.vault_root);
    let store = LocalObjectStore::open(&manifest.object_store.root)?;
    let vault_dirs = VAULT_DIRS
        .iter()
        .map(|dir| VaultDirStatus {
            path: (*dir).to_string(),
            exists: vault_root.join(dir).is_dir(),
        })
        .collect::<Vec<_>>();
    let vault_manifest_exists = store.exists("manifests/workspace.json")?;
    if vault_manifest_exists {
        let _ = store.read_manifest("manifests/workspace.json")?;
    }
    let manifest_object_count = store.list_prefix("manifests")?.len();
    let connection_count = manifest.connections.len();
    Ok(WorkspaceStatus {
        workspace: manifest,
        vault_dirs,
        vault_manifest_exists,
        manifest_object_count,
        connection_count,
    })
}

/// Record a read-only Gmail connection reference in the workspace vault.
pub fn connect_gmail(
    root: impl AsRef<Path>,
    mode: &str,
    auth_profile: &str,
) -> Result<ConnectionManifest> {
    let mut workspace = load_workspace(root)?;
    let mode = mode.trim();
    let mode = if mode.is_empty() { "readonly" } else { mode };
    let profile = auth_profile.trim();
    let profile = if profile.is_empty() {
        "default"
    } else {
        profile
    };
    let connection = ConnectionManifest {
        schema_version: CONNECTION_SCHEMA_VERSION.to_string(),
        provider: "gmail".to_string(),
        mode: mode.to_string(),
        auth_ref: format!("provider:gws:profile:{}", profile),
        scopes: vec!["gmail.readonly".to_string(), "drive.readonly".to_string()],
        workspace: workspace.name.clone(),
        secret_posture: "no credential material stored in workspace vault".to_string(),
        created_at: now_utc_string(),
    };
    let store = LocalObjectStore::open(&workspace.object_store.root)?;
    let manifest_path = "manifests/connections/gmail.json";
    store.write_manifest(manifest_path, &serde_json::to_value(&connection)?)?;

    workspace
        .connections
        .retain(|item| item.provider != "gmail");
    workspace.connections.push(ConnectionRef {
        provider: "gmail".to_string(),
        mode: mode.to_string(),
        manifest_path: manifest_path.to_string(),
    });
    write_workspace_manifest(&workspace)?;
    Ok(connection)
}

fn write_workspace_manifest(manifest: &WorkspaceManifest) -> Result<()> {
    let root = PathBuf::from(&manifest.workspace_root);
    let path = workspace_manifest_path(&root);
    fs::write(path, serde_json::to_vec_pretty(manifest)?)?;

    let store = LocalObjectStore::open(&manifest.object_store.root)?;
    store.write_manifest("manifests/workspace.json", &serde_json::to_value(manifest)?)?;
    let boundary = json!({
        "schema_version": "northroot.workspace_vault_boundary.v0",
        "workspace": manifest.name,
        "vault_root": manifest.vault_root,
        "object_store_backend": manifest.object_store.backend,
        "secret_posture": "secret values stay in external password managers or provider auth stores",
        "data_boundary": "raw and derived workspace data land in this vault"
    });
    store.write_manifest("manifests/vault-boundary.json", &boundary)?;
    let boundary_bytes = serde_json::to_vec_pretty(&boundary)?;
    let boundary_object = store.put("manifests/content", &boundary_bytes, "application/json")?;
    let stored_boundary = store.get(&boundary_object.path)?;
    if stored_boundary != boundary_bytes {
        return Err(WorkspaceError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "workspace vault boundary object verification failed",
        )));
    }
    Ok(())
}

fn workspace_manifest_path(root: &Path) -> PathBuf {
    root.join(WORKSPACE_DIR).join(WORKSPACE_MANIFEST)
}

fn now_utc_string() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("unix:{}", duration.as_secs()),
        Err(_) => "unix:0".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn init_creates_workspace_vault_layout() {
        let temp = TempDir::new().unwrap();
        let manifest = init_workspace("apd-local", temp.path()).unwrap();

        assert_eq!(manifest.name, "apd-local");
        assert_eq!(manifest.object_store.backend, "local");
        for dir in VAULT_DIRS {
            assert!(temp.path().join("vault").join(dir).is_dir(), "{dir}");
        }
        assert!(temp.path().join(".northroot/workspace.json").is_file());
        assert!(temp.path().join("vault/manifests/workspace.json").is_file());
    }

    #[test]
    fn status_reports_vault_dirs() {
        let temp = TempDir::new().unwrap();
        init_workspace("ops", temp.path()).unwrap();

        let status = workspace_status(temp.path()).unwrap();

        assert_eq!(status.workspace.name, "ops");
        assert!(status.vault_dirs.iter().all(|item| item.exists));
        assert!(status.vault_manifest_exists);
        assert!(status.manifest_object_count >= 3);
    }

    #[test]
    fn gmail_connection_manifest_contains_no_credentials() {
        let temp = TempDir::new().unwrap();
        init_workspace("ops", temp.path()).unwrap();

        let connection = connect_gmail(temp.path(), "readonly", "clearlyops_internal").unwrap();

        assert_eq!(connection.provider, "gmail");
        let serialized = serde_json::to_string(&connection).unwrap();
        assert!(!serialized.contains("token"));
        assert!(!serialized.contains("password"));
        assert!(!serialized.contains("secret_value"));
        assert!(serialized.contains("provider:gws:profile:clearlyops_internal"));
    }
}

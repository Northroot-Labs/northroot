//! Local object store facade for workspace vaults.

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as Sha2Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

/// Object store operation errors.
#[derive(Debug, Error)]
pub enum ObjectStoreError {
    /// An object path attempted to escape the store root.
    #[error("object path must be relative and traversal-free: {0}")]
    UnsafePath(String),
    /// The requested object does not exist.
    #[error("object not found: {0}")]
    NotFound(String),
    /// An I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization or parsing failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result alias for object store operations.
pub type Result<T> = std::result::Result<T, ObjectStoreError>;

/// Stored object metadata.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObjectMetadata {
    /// Stable object reference.
    pub object_ref: String,
    /// Media type supplied by the caller.
    pub media_type: String,
    /// Byte length of the stored object.
    pub byte_length: u64,
    /// Backend-relative path for the object bytes.
    pub path: String,
    /// SHA-256 digest encoded as base64url without padding.
    pub sha256_b64: String,
}

/// Minimal object store facade.
pub trait ObjectStore {
    /// Store immutable bytes and return content-addressed metadata.
    fn put(&self, prefix: &str, bytes: &[u8], media_type: &str) -> Result<ObjectMetadata>;
    /// Read an object by backend-relative path.
    fn get(&self, path: &str) -> Result<Vec<u8>>;
    /// Check whether a backend-relative path exists.
    fn exists(&self, path: &str) -> Result<bool>;
    /// List backend-relative paths under a prefix.
    fn list_prefix(&self, prefix: &str) -> Result<Vec<String>>;
    /// Write a named JSON manifest.
    fn write_manifest(&self, path: &str, value: &Value) -> Result<()>;
    /// Read a named JSON manifest.
    fn read_manifest(&self, path: &str) -> Result<Value>;
}

/// Local filesystem implementation of [`ObjectStore`].
#[derive(Clone, Debug)]
pub struct LocalObjectStore {
    root: PathBuf,
}

impl LocalObjectStore {
    /// Open or create a local object store at `root`.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    fn object_path(&self, relative: &str) -> Result<PathBuf> {
        Ok(self.root.join(safe_relative_path(relative)?))
    }
}

impl ObjectStore for LocalObjectStore {
    fn put(&self, prefix: &str, bytes: &[u8], media_type: &str) -> Result<ObjectMetadata> {
        let safe_prefix = safe_relative_path(prefix)?;
        let sha256_b64 = sha256_b64(bytes);
        let shard = &sha256_b64[..2];
        let object_ref = format!("sha256:{}", sha256_b64);
        let relative = safe_prefix.join("sha256").join(shard).join(&sha256_b64);
        let path = self.root.join(&relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            fs::write(&path, bytes)?;
        }
        let metadata = ObjectMetadata {
            object_ref,
            media_type: media_type.to_string(),
            byte_length: bytes.len() as u64,
            path: relative.to_string_lossy().replace('\\', "/"),
            sha256_b64,
        };
        let meta_path = path.with_extension("meta.json");
        fs::write(meta_path, serde_json::to_vec_pretty(&metadata)?)?;
        Ok(metadata)
    }

    fn get(&self, path: &str) -> Result<Vec<u8>> {
        let path = self.object_path(path)?;
        if !path.exists() {
            return Err(ObjectStoreError::NotFound(path.display().to_string()));
        }
        Ok(fs::read(path)?)
    }

    fn exists(&self, path: &str) -> Result<bool> {
        Ok(self.object_path(path)?.exists())
    }

    fn list_prefix(&self, prefix: &str) -> Result<Vec<String>> {
        let base = self.object_path(prefix)?;
        if !base.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        collect_files(&self.root, &base, &mut out)?;
        out.sort();
        Ok(out)
    }

    fn write_manifest(&self, path: &str, value: &Value) -> Result<()> {
        let path = self.object_path(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_vec_pretty(value)?)?;
        Ok(())
    }

    fn read_manifest(&self, path: &str) -> Result<Value> {
        let bytes = self.get(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

fn collect_files(root: &Path, current: &Path, out: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) != Some("json")
            || !path.to_string_lossy().ends_with(".meta.json")
        {
            if let Ok(relative) = path.strip_prefix(root) {
                out.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    Ok(())
}

fn sha256_b64(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn safe_relative_path(path: &str) -> Result<PathBuf> {
    let candidate = Path::new(path);
    if path.trim().is_empty() || candidate.is_absolute() {
        return Err(ObjectStoreError::UnsafePath(path.to_string()));
    }
    let mut out = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            _ => return Err(ObjectStoreError::UnsafePath(path.to_string())),
        }
    }
    if out.as_os_str().is_empty() {
        return Err(ObjectStoreError::UnsafePath(path.to_string()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn local_store_put_get_exists_and_list() {
        let temp = TempDir::new().unwrap();
        let store = LocalObjectStore::open(temp.path()).unwrap();

        let meta = store
            .put("raw", b"northroot workspace bytes", "text/plain")
            .unwrap();

        assert_eq!(meta.media_type, "text/plain");
        assert_eq!(store.get(&meta.path).unwrap(), b"northroot workspace bytes");
        assert!(store.exists(&meta.path).unwrap());
        assert_eq!(store.list_prefix("raw").unwrap(), vec![meta.path]);
    }

    #[test]
    fn local_store_rejects_traversal() {
        let temp = TempDir::new().unwrap();
        let store = LocalObjectStore::open(temp.path()).unwrap();

        assert!(store.put("../raw", b"x", "text/plain").is_err());
        assert!(store.get("../secret").is_err());
        assert!(store
            .write_manifest("/abs/manifest.json", &json!({}))
            .is_err());
    }

    #[test]
    fn local_store_writes_and_reads_named_manifest() {
        let temp = TempDir::new().unwrap();
        let store = LocalObjectStore::open(temp.path()).unwrap();
        let value = json!({
            "schema_version": "northroot.workspace_manifest.v0",
            "name": "apd-local"
        });

        store
            .write_manifest("manifests/workspace.json", &value)
            .unwrap();

        assert_eq!(
            store.read_manifest("manifests/workspace.json").unwrap(),
            value
        );
    }
}

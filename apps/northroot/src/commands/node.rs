//! Node custody and storage initialization commands.

use clap::Subcommand;
use northroot_node::{NodeManifest, NODE_MANIFEST_SCHEMA_V0};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const NODE_MANIFEST_FILE: &str = "node.json";

/// Node substrate subcommands.
#[derive(Subcommand)]
pub enum NodeCommand {
    /// Initialize a portable node root outside any repository
    Init {
        /// Node root directory. Defaults to $NORTHROOT_NODE_ROOT or ~/.northroot/node.
        #[arg(long)]
        root: Option<PathBuf>,
        /// Human-friendly alias. The generated node_id remains authoritative.
        #[arg(long)]
        slug: Option<String>,
        /// Custody/execution boundary represented by this node.
        #[arg(long, default_value = "user-machine")]
        scope: String,
        /// Replace an existing node manifest.
        #[arg(long)]
        force: bool,
        /// Pretty-print JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Inspect an initialized node root
    Status {
        /// Node root directory. Defaults to $NORTHROOT_NODE_ROOT or ~/.northroot/node.
        #[arg(long)]
        root: Option<PathBuf>,
        /// Pretty-print JSON output.
        #[arg(long)]
        json: bool,
    },
}

/// Runs a node substrate subcommand.
pub fn run(command: NodeCommand) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        NodeCommand::Init {
            root,
            slug,
            scope,
            force,
            json,
        } => init(root, slug, &scope, force, json),
        NodeCommand::Status { root, json } => status(root, json),
    }
}

fn init(
    root: Option<PathBuf>,
    slug: Option<String>,
    scope: &str,
    force: bool,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = resolve_node_root(root)?;
    let manifest_path = root.join(NODE_MANIFEST_FILE);
    if manifest_path.exists() && !force {
        return Err(format!(
            "node manifest already exists at {}; pass --force to replace it",
            manifest_path.display()
        )
        .into());
    }

    fs::create_dir_all(root.join("journal"))?;
    fs::create_dir_all(root.join("vault").join("objects"))?;
    fs::create_dir_all(root.join("state"))?;
    fs::create_dir_all(root.join("tmp"))?;

    let slug = slug.unwrap_or_else(default_node_slug);
    let manifest = NodeManifest::local(&slug, scope, &unique_suffix())?;
    write_manifest(&manifest_path, &manifest)?;

    let report = NodeStatusReport::from_manifest(root, manifest, true);
    emit_report(&report, json)?;
    Ok(())
}

fn status(root: Option<PathBuf>, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let root = resolve_node_root(root)?;
    let manifest_path = root.join(NODE_MANIFEST_FILE);
    let manifest = read_manifest(&manifest_path)?;
    manifest.validate()?;
    let report = NodeStatusReport::from_manifest(root, manifest, false);
    emit_report(&report, json)?;
    Ok(())
}

fn resolve_node_root(root: Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(root) = root {
        return Ok(root);
    }
    if let Ok(root) = std::env::var("NORTHROOT_NODE_ROOT") {
        if !root.trim().is_empty() {
            return Ok(PathBuf::from(root));
        }
    }
    let home = std::env::var("HOME").map_err(|_| "HOME is not set; pass --root")?;
    Ok(PathBuf::from(home).join(".northroot").join("node"))
}

fn write_manifest(path: &Path, manifest: &NodeManifest) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = serde_json::to_vec_pretty(manifest)?;
    fs::write(path, [bytes, b"\n".to_vec()].concat())?;
    Ok(())
}

fn read_manifest(path: &Path) -> Result<NodeManifest, Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn emit_report(report: &NodeStatusReport, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        println!("node_id: {}", report.node_id);
        println!("slug: {}", report.slug.as_deref().unwrap_or("-"));
        println!("scope: {}", report.scope.as_deref().unwrap_or("-"));
        println!("root: {}", report.root);
        println!("manifest: {}", report.manifest_path);
        println!("index: {}", report.index_uri.as_deref().unwrap_or("-"));
        println!("primary_object_store: {}", report.primary_object_store_uri.as_deref().unwrap_or("-"));
        println!("exists: {}", report.exists);
        println!("created: {}", report.created);
    }
    Ok(())
}

fn default_node_slug() -> String {
    std::env::var("USER")
        .ok()
        .and_then(|user| sanitize_slug(&user))
        .unwrap_or_else(|| "local-node".to_string())
}

fn sanitize_slug(value: &str) -> Option<String> {
    let mut slug = String::new();
    let mut previous_dash = false;
    for byte in value.bytes() {
        let next = if byte.is_ascii_alphanumeric() {
            Some(byte.to_ascii_lowercase() as char)
        } else if byte == b'-' || byte == b'_' || byte == b'.' {
            Some('-')
        } else {
            None
        };
        if let Some(ch) = next {
            if ch == '-' {
                if previous_dash {
                    continue;
                }
                previous_dash = true;
            } else {
                previous_dash = false;
            }
            slug.push(ch);
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        None
    } else {
        Some(slug)
    }
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{nanos:x}-{:x}", std::process::id())
}

#[derive(Serialize)]
struct NodeStatusReport {
    schema: &'static str,
    exists: bool,
    created: bool,
    root: String,
    manifest_path: String,
    node_id: String,
    slug: Option<String>,
    scope: Option<String>,
    index_uri: Option<String>,
    primary_object_store_uri: Option<String>,
    journal_path: String,
    vault_path: String,
    state_path: String,
}

impl NodeStatusReport {
    fn from_manifest(root: PathBuf, manifest: NodeManifest, created: bool) -> Self {
        let manifest_path = root.join(NODE_MANIFEST_FILE);
        let primary_object_store_uri = manifest
            .object_stores
            .iter()
            .find(|store| store.role == "primary")
            .map(|store| store.uri.clone());
        Self {
            schema: NODE_MANIFEST_SCHEMA_V0,
            exists: manifest_path.exists(),
            created,
            root: root.display().to_string(),
            manifest_path: manifest_path.display().to_string(),
            node_id: manifest.node_id,
            slug: manifest.slug,
            scope: manifest.scope,
            index_uri: manifest.index.map(|index| index.uri),
            primary_object_store_uri,
            journal_path: manifest.journal_path,
            vault_path: manifest.vault_path,
            state_path: manifest.state_path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn init_creates_portable_node_root() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("node");

        init(
            Some(root.clone()),
            Some("demo-node".to_string()),
            "user-machine",
            false,
            true,
        )
        .unwrap();

        assert!(root.join("node.json").is_file());
        assert!(root.join("journal").is_dir());
        assert!(root.join("vault").join("objects").is_dir());
        assert!(root.join("state").is_dir());
        let manifest = read_manifest(&root.join("node.json")).unwrap();
        assert_eq!(manifest.slug.as_deref(), Some("demo-node"));
        assert_eq!(
            manifest.index.as_ref().map(|index| index.uri.as_str()),
            Some("sqlite://state/node.db")
        );
        assert_eq!(manifest.object_stores[0].uri, "fs://vault/objects");
        manifest.validate().unwrap();
    }

    #[test]
    fn init_rejects_existing_manifest_without_force() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("node");

        init(
            Some(root.clone()),
            Some("demo-node".to_string()),
            "user-machine",
            false,
            true,
        )
        .unwrap();

        let result = init(
            Some(root),
            Some("demo-node".to_string()),
            "user-machine",
            false,
            true,
        );
        assert!(result.is_err());
    }

    #[test]
    fn status_validates_manifest() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("node");
        init(
            Some(root.clone()),
            Some("demo-node".to_string()),
            "user-machine",
            false,
            true,
        )
        .unwrap();

        status(Some(root), true).unwrap();
    }
}

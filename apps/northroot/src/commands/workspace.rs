//! Workspace CLI commands.

use crate::workspace;
use std::path::PathBuf;

/// Initialize a local Northroot workspace and workspace vault.
pub fn init(name: String, root: String) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = workspace::init_workspace(&name, PathBuf::from(root))?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}

/// Print local Northroot workspace status.
pub fn status(root: String) -> Result<(), Box<dyn std::error::Error>> {
    let status = workspace::workspace_status(PathBuf::from(root))?;
    println!("{}", serde_json::to_string_pretty(&status)?);
    Ok(())
}

//! Provider connection CLI commands.

use crate::workspace;
use std::path::PathBuf;

/// Record a read-only Gmail connection reference for a workspace.
pub fn gmail(
    workspace_root: String,
    mode: String,
    profile: String,
) -> Result<(), Box<dyn std::error::Error>> {
    if mode != "readonly" {
        return Err("gmail connection v0 only supports --mode readonly".into());
    }
    let connection = workspace::connect_gmail(PathBuf::from(workspace_root), &mode, &profile)?;
    println!("{}", serde_json::to_string_pretty(&connection)?);
    Ok(())
}

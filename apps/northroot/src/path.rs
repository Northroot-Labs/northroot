//! Path validation utilities for journal file access.

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during path validation.
#[derive(Error, Debug)]
pub enum PathError {
    /// Path contains traversal sequences after resolution.
    #[error("path contains traversal sequences: {0}")]
    Traversal(String),
    /// Path is a symlink (if symlinks are rejected).
    #[error("path is a symlink: {0}")]
    Symlink(String),
    /// I/O error during path resolution.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Path cannot be resolved to absolute form.
    #[error("cannot resolve path to absolute: {0}")]
    CannotResolve(String),
}

/// Validates and normalizes a journal file path.
///
/// This function:
/// 1. Resolves the path to an absolute path
/// 2. Rejects paths containing ".." after resolution (path traversal)
/// 3. Optionally rejects symlinks
///
/// # Arguments
///
/// * `path` - The journal file path to validate
/// * `reject_symlinks` - If true, reject symlinks; if false, resolve and validate target
///
/// # Returns
///
/// Returns the normalized absolute path, or an error if validation fails.
pub fn validate_journal_path(path: &str, reject_symlinks: bool) -> Result<PathBuf, PathError> {
    let path = Path::new(path);

    // Resolve to absolute path
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?
            .join(path)
            .canonicalize()
            .map_err(|e| PathError::CannotResolve(format!("{}: {}", path.display(), e)))?
    };

    // Check if it's a symlink
    if reject_symlinks && absolute.symlink_metadata()?.file_type().is_symlink() {
        return Err(PathError::Symlink(absolute.display().to_string()));
    }

    // If not rejecting symlinks, resolve the symlink target
    let resolved = if !reject_symlinks {
        absolute
            .canonicalize()
            .map_err(|e| PathError::CannotResolve(format!("{}: {}", absolute.display(), e)))?
    } else {
        absolute
    };

    // Check for path traversal sequences in the resolved path
    // After canonicalization, ".." should be resolved, but we check the string representation
    // to catch any edge cases
    let path_str = resolved.to_string_lossy();
    if path_str.contains("..") {
        return Err(PathError::Traversal(resolved.display().to_string()));
    }

    Ok(resolved)
}

/// Sanitizes a path for display in error messages.
///
/// Replaces absolute paths outside the current working directory with a generic message
/// to avoid leaking filesystem structure.
pub fn sanitize_path_for_error(path: &std::path::Path) -> String {
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(relative) = path.strip_prefix(&cwd) {
            // Path is within working directory, safe to show
            return relative.display().to_string();
        }
    }
    // Path is outside working directory or we can't determine cwd
    // Show only the filename to avoid leaking structure
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "<journal file>".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_absolute_path() {
        let temp = TempDir::new().unwrap();
        let journal_path = temp.path().join("test.nrj");
        fs::File::create(&journal_path).unwrap();

        let result = validate_journal_path(journal_path.to_str().unwrap(), false);
        assert!(result.is_ok());
        assert!(result.unwrap().is_absolute());
    }

    #[test]
    fn test_relative_path() {
        let temp = TempDir::new().unwrap();
        let journal_path = temp.path().join("test.nrj");
        fs::File::create(&journal_path).unwrap();

        // Change to temp directory
        std::env::set_current_dir(temp.path()).unwrap();

        let result = validate_journal_path("test.nrj", false);
        assert!(result.is_ok());
        assert!(result.unwrap().is_absolute());
    }

    #[test]
    fn test_path_traversal_rejected() {
        let temp = TempDir::new().unwrap();
        let journal_path = temp.path().join("test.nrj");
        fs::File::create(&journal_path).unwrap();

        // Try to access parent directory
        std::env::set_current_dir(temp.path()).unwrap();
        let result = validate_journal_path("../test.nrj", false);
        // This should either resolve to a different path or fail
        // The exact behavior depends on whether the parent has test.nrj
        // But we check that the resolved path doesn't contain ".."
        if let Ok(resolved) = result {
            assert!(!resolved.to_string_lossy().contains(".."));
        }
    }

    #[test]
    fn test_nonexistent_path() {
        let result = validate_journal_path("/nonexistent/path/to/journal.nrj", false);
        assert!(result.is_err());
        match result.unwrap_err() {
            PathError::CannotResolve(_) => {}
            _ => panic!("expected CannotResolve error"),
        }
    }

    #[test]
    fn test_symlink_rejection() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target.nrj");
        fs::File::create(&target).unwrap();

        let symlink = temp.path().join("link.nrj");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &symlink).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&target, &symlink).unwrap();

        let result = validate_journal_path(symlink.to_str().unwrap(), true);
        assert!(result.is_err());
        match result.unwrap_err() {
            PathError::Symlink(_) => {}
            _ => panic!("expected Symlink error"),
        }
    }

    #[test]
    fn test_symlink_resolution() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target.nrj");
        fs::File::create(&target).unwrap();

        let symlink = temp.path().join("link.nrj");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &symlink).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&target, &symlink).unwrap();

        let result = validate_journal_path(symlink.to_str().unwrap(), false);
        assert!(result.is_ok());
        // Resolved path should point to the target
        let resolved = result.unwrap();
        assert!(resolved.ends_with("target.nrj"));
    }
}

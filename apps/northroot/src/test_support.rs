//! Shared test helpers for process-global state.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

static CWD_LOCK: Mutex<()> = Mutex::new(());

/// Guard that serializes tests which mutate the process current directory.
pub struct CwdGuard {
    original: PathBuf,
    _lock: MutexGuard<'static, ()>,
}

impl CwdGuard {
    /// Enters `path` as the process current directory until the guard is dropped.
    pub fn enter(path: &Path) -> Self {
        let lock = CWD_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(path).unwrap();
        Self {
            original,
            _lock: lock,
        }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}

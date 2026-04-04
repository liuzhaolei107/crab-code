//! File locking via `fd-lock`.

use std::path::Path;

/// A held file lock. Dropped when the guard goes out of scope.
pub struct FileLockGuard {
    _private: (),
}

/// Acquire an exclusive lock on `path`, creating the file if needed.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or locked.
pub fn lock_exclusive(_path: &Path) -> crab_common::Result<FileLockGuard> {
    todo!()
}

/// Try to acquire an exclusive lock without blocking. Returns `None` if already held.
///
/// # Errors
///
/// Returns an error if the file cannot be opened.
pub fn try_lock_exclusive(_path: &Path) -> crab_common::Result<Option<FileLockGuard>> {
    todo!()
}

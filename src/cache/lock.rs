//! Backoff lock file: holds a `blocked_until` epoch + error kind.

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::cache::atomic_helper;

/// The kind of error that caused the lock to be set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockError {
    RateLimited,
    AuthFailed,
    Network,
}

/// A backoff lock: prevents further API calls until `blocked_until` elapses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Lock {
    /// Unix epoch seconds until which API calls should be skipped.
    pub blocked_until: u64,
    /// The error kind that triggered this lock.
    pub error: LockError,
}

/// Read the lock file at `path`.
///
/// Returns `None` on any error: file missing, permission denied, malformed JSON.
/// Never panics.
pub fn read(path: &Path) -> Option<Lock> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Write `lock` to `path` atomically via the atomic-helper.
pub fn write(path: &Path, lock: &Lock) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(lock)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    atomic_helper::write_atomic(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn make_lock(blocked_until: u64, error: LockError) -> Lock {
        Lock {
            blocked_until,
            error,
        }
    }

    #[test]
    fn read_missing_path_returns_none() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.lock");
        assert_eq!(read(&path), None);
    }

    #[test]
    fn read_malformed_json_returns_none() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.lock");
        fs::write(&path, b"not valid json { garbage").unwrap();
        assert_eq!(read(&path), None);
    }

    #[test]
    fn read_empty_file_returns_none() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.lock");
        fs::write(&path, b"").unwrap();
        assert_eq!(read(&path), None);
    }

    #[test]
    fn read_valid_lock_returns_some() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("valid.lock");
        let json = r#"{"blocked_until":9999999999,"error":"RateLimited"}"#;
        fs::write(&path, json.as_bytes()).unwrap();
        let lock = read(&path).expect("should parse valid lock");
        assert_eq!(lock.blocked_until, 9_999_999_999);
        assert_eq!(lock.error, LockError::RateLimited);
    }

    #[test]
    fn write_then_read_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("round_trip.lock");

        let original = make_lock(1_700_000_000, LockError::AuthFailed);
        write(&path, &original).unwrap();

        let recovered = read(&path).expect("round-trip read must succeed");
        assert_eq!(recovered, original);
    }

    #[test]
    fn round_trip_rate_limited() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rl.lock");
        let lock = make_lock(1_000_000, LockError::RateLimited);
        write(&path, &lock).unwrap();
        assert_eq!(read(&path), Some(lock));
    }

    #[test]
    fn round_trip_network() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("net.lock");
        let lock = make_lock(42, LockError::Network);
        write(&path, &lock).unwrap();
        assert_eq!(read(&path), Some(lock));
    }

    /// lock.rs simply stores and retrieves data; freshness / expiry logic
    /// lives in the Task-9 orchestrator, not here.
    #[test]
    fn lock_rs_does_not_check_expiry() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("expiry.lock");

        // A lock with blocked_until = 0 (already "expired" in epoch time).
        let expired = make_lock(0, LockError::Network);
        write(&path, &expired).unwrap();

        // lock.rs returns it as-is; freshness check is the caller's job.
        let recovered = read(&path).expect("lock.rs should not discard expired locks");
        assert_eq!(recovered.blocked_until, 0);

        // A lock far in the future — also just round-trips.
        let active = make_lock(u64::MAX, LockError::AuthFailed);
        write(&path, &active).unwrap();
        let recovered2 = read(&path).unwrap();
        assert_eq!(recovered2.blocked_until, u64::MAX);
    }
}

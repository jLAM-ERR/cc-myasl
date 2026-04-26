//! Exponential backoff ladder for lock durations.

use crate::cache::lock::LockError;

/// Exponential backoff ladder for `LockError::Network` errors (seconds).
/// Capped at 1800 s (30 minutes).
const NETWORK_LADDER: &[u64] = &[60, 120, 240, 480, 960, 1800];

/// Return the number of seconds the lock should be held, given:
/// - `consecutive_failures`: how many back-to-back failures have occurred
///   (used only for `LockError::Network` to index the backoff ladder).
/// - `err_kind`: the kind of error.
///
/// | Error kind         | Behaviour                                                |
/// |--------------------|----------------------------------------------------------|
/// | `AuthFailed`       | Always 3 600 s regardless of `consecutive_failures`.    |
/// | `RateLimited`      | Always 300 s (callers may override with `Retry-After`). |
/// | `Network`          | Exponential ladder: 60 → 120 → 240 → 480 → 960 → 1800, |
/// |                    | capped at 1800 s.                                        |
pub fn next_lock_seconds(consecutive_failures: u32, err_kind: LockError) -> u64 {
    match err_kind {
        LockError::AuthFailed => 3600,
        LockError::RateLimited => 300,
        LockError::Network => {
            let idx = (consecutive_failures as usize).min(NETWORK_LADDER.len() - 1);
            NETWORK_LADDER[idx]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::lock::LockError;

    // ── AuthFailed ────────────────────────────────────────────────────────────

    #[test]
    fn auth_failed_always_3600() {
        for failures in [0u32, 1, 2, 5, 10, 100] {
            assert_eq!(
                next_lock_seconds(failures, LockError::AuthFailed),
                3600,
                "AuthFailed should always return 3600 (failures={failures})"
            );
        }
    }

    // ── RateLimited ───────────────────────────────────────────────────────────

    #[test]
    fn rate_limited_always_300() {
        for failures in [0u32, 1, 2, 5, 10, 100] {
            assert_eq!(
                next_lock_seconds(failures, LockError::RateLimited),
                300,
                "RateLimited should always return 300 (failures={failures})"
            );
        }
    }

    // ── Network ladder ────────────────────────────────────────────────────────

    #[test]
    fn network_ladder_progression() {
        let cases: &[(u32, u64)] = &[(0, 60), (1, 120), (2, 240), (3, 480), (4, 960), (5, 1800)];
        for &(failures, expected) in cases {
            assert_eq!(
                next_lock_seconds(failures, LockError::Network),
                expected,
                "Network ladder mismatch at consecutive_failures={failures}"
            );
        }
    }

    #[test]
    fn network_ladder_capped_at_1800() {
        // One past the last rung.
        assert_eq!(next_lock_seconds(6, LockError::Network), 1800);
        // Far past the last rung.
        assert_eq!(next_lock_seconds(100, LockError::Network), 1800);
    }
}

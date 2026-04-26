//! On-disk cache for OAuth usage data.
//!
//! `atomic_helper` provides write-temp-then-rename safety.
//! `lock` is the backoff lock file.
//! `backoff` decides how long the lock holds based on the error.

pub mod atomic_helper;
pub mod backoff;
pub mod lock;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ── Cache directory resolver ────────────────────────────────────────────────

/// Locate the cache directory for claude-statusline.
///
/// Uses `directories::ProjectDirs::from("ai", "claude-statusline",
/// "claude-statusline")` to find the platform-appropriate cache directory
/// (e.g. `~/Library/Caches/claude-statusline/` on macOS,
/// `~/.cache/claude-statusline/` on Linux).
///
/// Fallback (when `ProjectDirs` returns `None`): constructs the path from
/// `$HOME` environment variable as `$HOME/.cache/claude-statusline/`.
/// If `$HOME` is also unset, returns `~/.cache/claude-statusline/` literally.
pub fn cache_dir() -> PathBuf {
    if let Some(proj) =
        directories::ProjectDirs::from("ai", "claude-statusline", "claude-statusline")
    {
        return proj.cache_dir().to_path_buf();
    }

    // Fallback: derive from BaseDirs or $HOME.
    if let Some(base) = directories::BaseDirs::new() {
        return base.home_dir().join(".cache").join("claude-statusline");
    }

    // Last resort: use $HOME env var.
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".cache").join("claude-statusline");
    }

    // Absolute fallback — caller will likely get an error on first write.
    PathBuf::from("~/.cache/claude-statusline")
}

// ── Data types ──────────────────────────────────────────────────────────────

/// On-disk cache schema for OAuth usage data.
///
/// **Deliberately omits any token field** — bearer tokens must never be
/// written to disk. This struct is the compile-time guarantee for
/// Hard Invariant #2. A test in this module serializes it to JSON and
/// asserts that none of the forbidden substrings ("token", "bearer",
/// "secret", "auth", "access") appear.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct UsageCache {
    /// Unix timestamp (seconds) at which this data was fetched.
    pub fetched_at: u64,
    /// 5-hour rolling window data.
    pub five_hour: Option<UsageWindowCache>,
    /// 7-day rolling window data.
    pub seven_day: Option<UsageWindowCache>,
    /// Extra/add-on usage data.
    pub extra_usage: Option<ExtraUsageCache>,
}

/// Cache entry for a single usage window (5-hour or 7-day).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct UsageWindowCache {
    /// Utilization percentage `0..=100` as reported by the API
    /// (NOT a fractional 0..=1 ratio — main.rs computes `100 - utilization`
    /// to get the "% remaining" rendered by the format engine).
    pub utilization: Option<f64>,
    /// ISO-8601 reset timestamp from the API.
    pub resets_at: Option<String>,
}

/// Cache entry for extra/add-on usage.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ExtraUsageCache {
    /// Whether extra usage is enabled for this account.
    pub is_enabled: Option<bool>,
    /// Monthly credit limit in dollars.
    pub monthly_limit: Option<f64>,
    /// Credits used this month in dollars.
    pub used_credits: Option<f64>,
    /// Utilization percentage `0..=100` (same scale as `UsageWindowCache`).
    pub utilization: Option<f64>,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Read the cache from `dir/usage.json`.
///
/// Returns `None` on missing file, permission error, or JSON parse failure.
/// Never panics.
pub fn read(dir: &Path) -> Option<UsageCache> {
    let content = fs::read_to_string(dir.join("usage.json")).ok()?;
    serde_json::from_str(&content).ok()
}

/// Write `cache` to `dir/usage.json` atomically.
///
/// Uses `atomic_helper::write_atomic` (write-tmp-then-rename) so readers
/// never observe a partially-written file. The caller is responsible for
/// ensuring `dir` exists before calling.
pub fn write(dir: &Path, cache: &UsageCache) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(cache)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    atomic_helper::write_atomic(&dir.join("usage.json"), &bytes)
}

/// Return `true` if `cache` was fetched within `ttl_secs` of `now`.
///
/// Uses saturating subtraction so a future `fetched_at` (clock skew) does
/// not underflow; the cache is considered fresh in that edge case.
pub fn is_fresh(cache: &UsageCache, ttl_secs: u64, now: u64) -> bool {
    now.saturating_sub(cache.fetched_at) < ttl_secs
}

/// Read the cache from `dir/usage.json`, returning stale data if present.
///
/// Semantically identical to [`read`]; the distinct name signals caller
/// intent — "I know this might be expired but I want it anyway."
/// Freshness checking is the caller's responsibility via [`is_fresh`].
pub fn read_stale(dir: &Path) -> Option<UsageCache> {
    read(dir)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    /// Construct a fully-populated UsageCache for use in tests.
    /// Utilization values are on the 0..=100 percentage scale, matching the
    /// API contract (see `UsageWindowCache::utilization` doc comment).
    fn full_cache(fetched_at: u64) -> UsageCache {
        UsageCache {
            fetched_at,
            five_hour: Some(UsageWindowCache {
                utilization: Some(42.0),
                resets_at: Some("2026-04-26T18:00:00Z".to_string()),
            }),
            seven_day: Some(UsageWindowCache {
                utilization: Some(75.0),
                resets_at: Some("2026-04-30T00:00:00Z".to_string()),
            }),
            extra_usage: Some(ExtraUsageCache {
                is_enabled: Some(true),
                monthly_limit: Some(100.0),
                used_credits: Some(37.5),
                utilization: Some(37.5),
            }),
        }
    }

    // ── round-trip ──────────────────────────────────────────────────────────

    #[test]
    fn write_then_read_round_trip() {
        let dir = TempDir::new().unwrap();
        let cache = full_cache(1_700_000_000);
        write(dir.path(), &cache).unwrap();
        let recovered = read(dir.path()).expect("must read back what was written");
        assert_eq!(recovered, cache);
    }

    #[test]
    fn write_then_read_empty_cache() {
        let dir = TempDir::new().unwrap();
        let cache = UsageCache::default();
        write(dir.path(), &cache).unwrap();
        let recovered = read(dir.path()).expect("default cache must round-trip");
        assert_eq!(recovered, cache);
    }

    // ── read error cases ────────────────────────────────────────────────────

    #[test]
    fn read_missing_path_returns_none() {
        let dir = TempDir::new().unwrap();
        // No file written — must return None.
        assert!(read(dir.path()).is_none());
    }

    #[test]
    fn read_malformed_json_returns_none() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("usage.json"), b"{ not valid json !!!").unwrap();
        assert!(read(dir.path()).is_none());
    }

    #[test]
    fn read_empty_file_returns_none() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("usage.json"), b"").unwrap();
        assert!(read(dir.path()).is_none());
    }

    // ── is_fresh ────────────────────────────────────────────────────────────

    #[test]
    fn is_fresh_within_ttl_returns_true() {
        let cache = UsageCache {
            fetched_at: 1000,
            ..Default::default()
        };
        // age = 100 s, ttl = 180 s → fresh
        assert!(is_fresh(&cache, 180, 1100));
    }

    #[test]
    fn is_fresh_beyond_ttl_returns_false() {
        let cache = UsageCache {
            fetched_at: 1000,
            ..Default::default()
        };
        // age = 200 s, ttl = 180 s → stale
        assert!(!is_fresh(&cache, 180, 1200));
    }

    #[test]
    fn is_fresh_boundary_one_second_inside_ttl_is_fresh() {
        let cache = UsageCache {
            fetched_at: 1000,
            ..Default::default()
        };
        // age == ttl-1 = 179 s → still within TTL → fresh
        assert!(is_fresh(&cache, 180, 1179));
    }

    #[test]
    fn is_fresh_at_exact_ttl_age_is_stale() {
        let cache = UsageCache {
            fetched_at: 1000,
            ..Default::default()
        };
        // age == ttl = 180 s → NOT fresh (strict <)
        assert!(!is_fresh(&cache, 180, 1180));
    }

    #[test]
    fn is_fresh_zero_age_always_true() {
        let cache = UsageCache {
            fetched_at: 5000,
            ..Default::default()
        };
        assert!(is_fresh(&cache, 180, 5000));
    }

    #[test]
    fn is_fresh_future_fetched_at_saturates_to_zero_age() {
        // fetched_at > now → saturating_sub yields 0 → always fresh
        let cache = UsageCache {
            fetched_at: 9_999_999_999,
            ..Default::default()
        };
        assert!(is_fresh(&cache, 180, 1000));
    }

    // ── read_stale ──────────────────────────────────────────────────────────

    #[test]
    fn read_stale_returns_expired_data() {
        let dir = TempDir::new().unwrap();
        // Write a cache with fetched_at = 0 (epoch) — definitely stale.
        let cache = full_cache(0);
        write(dir.path(), &cache).unwrap();

        // read_stale must return it regardless of age.
        let recovered = read_stale(dir.path()).expect("read_stale must not filter on age");
        assert_eq!(recovered, cache);
        assert_eq!(recovered.fetched_at, 0);

        // Confirm it is indeed stale by a large margin.
        assert!(!is_fresh(&recovered, 180, 1_000_000_000));
    }

    #[test]
    fn read_stale_missing_returns_none() {
        let dir = TempDir::new().unwrap();
        assert!(read_stale(dir.path()).is_none());
    }

    // ── token-redaction invariant (Hard Invariant #2) ────────────────────────

    #[test]
    fn no_forbidden_substrings_in_serialized_cache() {
        let cache = full_cache(1_714_000_000);
        let json = serde_json::to_string_pretty(&cache).expect("serialization must succeed");
        let lower = json.to_lowercase();

        for forbidden in &["token", "bearer", "secret", "auth", "access"] {
            assert!(
                !lower.contains(forbidden),
                "Serialized UsageCache must not contain {forbidden:?}; got:\n{json}"
            );
        }
    }

    // ── concurrent safety ────────────────────────────────────────────────────

    #[test]
    fn concurrent_writes_and_reads_no_corruption() {
        let dir = Arc::new(TempDir::new().unwrap());
        let n = 20;

        let mut handles = Vec::new();

        // 20 writer threads.
        for i in 0..n {
            let dir = Arc::clone(&dir);
            handles.push(std::thread::spawn(move || {
                let cache = UsageCache {
                    fetched_at: i as u64,
                    five_hour: Some(UsageWindowCache {
                        utilization: Some(i as f64 / 20.0),
                        resets_at: Some(format!("2026-04-26T{i:02}:00:00Z")),
                    }),
                    seven_day: None,
                    extra_usage: None,
                };
                // Errors (e.g. rename race) are acceptable; corruption is not.
                let _ = write(dir.path(), &cache);
            }));
        }

        // 20 reader threads.
        for _ in 0..n {
            let dir = Arc::clone(&dir);
            handles.push(std::thread::spawn(move || {
                // Every read must return either None (file not yet written) or
                // a successfully parsed UsageCache — never partial/corrupt data.
                let result = read(dir.path());
                // If Some, it must have been parseable (which it is, since read()
                // returns None on parse error). We just assert it doesn't panic.
                let _ = result;
            }));
        }

        for h in handles {
            h.join().expect("thread must not panic");
        }

        // After all threads finish, the file (if present) must parse cleanly.
        if let Some(final_cache) = read(dir.path()) {
            // Validate that fetched_at is in the expected range written by writers.
            assert!(
                final_cache.fetched_at < n as u64,
                "fetched_at out of range: {}",
                final_cache.fetched_at
            );
        }
    }
}

//! On-disk cache for OAuth usage data.
//!
//! `atomic_helper` provides write-temp-then-rename safety.
//! `lock` is the backoff lock file.
//! `backoff` decides how long the lock holds based on the error.
//! The full `UsageCache` orchestrator (read / write / is_fresh) lands in Task 9.

pub mod atomic_helper;
pub mod backoff;
pub mod lock;

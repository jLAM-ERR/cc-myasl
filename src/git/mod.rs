use std::path::{Path, PathBuf};

use gix::bstr::ByteSlice;

/// Owns a gix repository handle.
pub struct Repo {
    inner: gix::Repository,
}

/// Walk parent directories from `start` looking for a git repo; returns None on failure.
pub fn discover(start: &Path) -> Option<Repo> {
    gix::discover(start).ok().map(|r| Repo { inner: r })
}

impl Repo {
    /// Short branch name (e.g. "main"); None for detached HEAD or unborn.
    pub fn branch(&self) -> Option<String> {
        self.inner
            .head_name()
            .ok()
            .flatten()
            .map(|full| full.shorten().to_str_lossy().into_owned())
    }

    /// Worktree root directory; None for bare repositories.
    pub fn root(&self) -> Option<PathBuf> {
        self.inner.work_dir().map(PathBuf::from)
    }
}

/// Serialises tests that read or write `GIT_CEILING_DIRECTORIES` or other git env vars.
#[cfg(test)]
pub(crate) static GIT_ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

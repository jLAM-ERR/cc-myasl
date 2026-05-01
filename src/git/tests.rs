use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tempfile::TempDir;

/// Creates a temporary git repo with `git init -b main` (falling back to `git init`
/// + rename if -b is unsupported), and returns the TempDir handle.
fn init_repo(branch: &str) -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "init", "-b", branch])
        .output()
        .expect("git init");
    if !out.status.success() {
        // Older git without -b support; init then rename.
        Command::new("git")
            .args(["-C", dir.path().to_str().unwrap(), "init"])
            .status()
            .expect("git init fallback");
        Command::new("git")
            .args(["-C", dir.path().to_str().unwrap(), "checkout", "-b", branch])
            .stderr(Stdio::null())
            .status()
            .expect("git checkout -b failed");
    }
    dir
}

/// Creates a commit in the repo so HEAD is no longer unborn.
fn commit(dir: &TempDir) {
    let path = dir.path().join("f");
    std::fs::write(&path, b"x").expect("write file");
    Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "add", "."])
        .status()
        .expect("git add");
    Command::new("git")
        .args([
            "-C",
            dir.path().to_str().unwrap(),
            "-c",
            "user.email=t@t.com",
            "-c",
            "user.name=T",
            "commit",
            "-m",
            "init",
        ])
        .status()
        .expect("git commit");
}

// ── decoupling invariant ─────────────────────────────────────────────────

/// Walk `src/git/` and assert no `.rs` file contains forbidden high-level imports.
#[test]
fn git_module_does_not_depend_on_format_config_api_cache() {
    use std::fs;

    let forbidden: &[&str] = &[
        &["use crate", "::", "format"].concat(),
        &["use crate", "::", "config"].concat(),
        &["use crate", "::", "api"].concat(),
        &["use crate", "::", "cache"].concat(),
    ];

    fn walk(dir: &Path, forbidden: &[&str]) {
        let entries = fs::read_dir(dir).expect("read_dir failed");
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, forbidden);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let src = fs::read_to_string(&path).unwrap_or_default();
                for pat in forbidden {
                    assert!(
                        !src.contains(*pat),
                        "{} has forbidden import: {}",
                        path.display(),
                        pat
                    );
                }
            }
        }
    }

    walk(Path::new("src/git"), forbidden);
}

#[test]
fn discover_returns_some_inside_repo() {
    let dir = init_repo("main");
    assert!(super::discover(dir.path()).is_some());
}

#[test]
fn discover_returns_none_outside_repo() {
    let _guard = super::GIT_ENV_MUTEX.lock().unwrap();
    let dir = tempfile::tempdir().expect("tempdir");
    // Prevent gix from walking past the tempdir even if a parent is a git repo.
    std::env::set_var("GIT_CEILING_DIRECTORIES", dir.path());
    let result = super::discover(dir.path());
    std::env::remove_var("GIT_CEILING_DIRECTORIES");
    assert!(result.is_none());
}

#[test]
fn branch_returns_main_after_initial_commit() {
    let dir = init_repo("main");
    commit(&dir);
    let repo = super::discover(dir.path()).expect("repo");
    assert_eq!(repo.branch(), Some("main".to_owned()));
}

#[test]
fn branch_returns_none_for_detached_head() {
    let dir = init_repo("main");
    commit(&dir);
    // Detach HEAD by checking out the commit hash directly.
    let out = Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "rev-parse", "HEAD"])
        .output()
        .expect("rev-parse");
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    Command::new("git")
        .args([
            "-C",
            dir.path().to_str().unwrap(),
            "checkout",
            "--quiet",
            &sha,
        ])
        .status()
        .expect("git checkout sha");
    let repo = super::discover(dir.path()).expect("repo");
    assert_eq!(repo.branch(), None);
}

#[test]
fn root_returns_worktree_path() {
    let dir = init_repo("main");
    let repo = super::discover(dir.path()).expect("repo");
    let root = repo.root().expect("root");
    // Canonicalise both sides to resolve any symlinks (macOS /var → /private/var).
    let expected: PathBuf = dir.path().canonicalize().expect("canonicalize");
    let got: PathBuf = root.canonicalize().expect("canonicalize root");
    assert_eq!(got, expected);
}

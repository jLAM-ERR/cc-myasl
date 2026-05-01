use super::*;
use std::path::PathBuf;

fn ctx_empty() -> RenderCtx {
    RenderCtx::default()
}

// ── compress_home helper ──────────────────────────────────────────────────

#[test]
fn compress_home_replaces_prefix() {
    let _guard = crate::creds::HOME_MUTEX.lock().unwrap();
    let saved = std::env::var("HOME").ok();
    unsafe { std::env::set_var("HOME", "/home/alice") };
    let result = compress_home(std::path::Path::new("/home/alice/projects/foo"));
    match saved {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    assert_eq!(result, Some("~/projects/foo".to_owned()));
}

#[test]
fn compress_home_path_not_under_home_unchanged() {
    let _guard = crate::creds::HOME_MUTEX.lock().unwrap();
    let saved = std::env::var("HOME").ok();
    unsafe { std::env::set_var("HOME", "/home/alice") };
    let result = compress_home(std::path::Path::new("/var/data/project"));
    match saved {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    assert_eq!(result, Some("/var/data/project".to_owned()));
}

#[test]
fn compress_home_empty_home_var_no_substitution() {
    let _guard = crate::creds::HOME_MUTEX.lock().unwrap();
    let saved = std::env::var("HOME").ok();
    unsafe { std::env::set_var("HOME", "") };
    let result = compress_home(std::path::Path::new("/home/alice/foo"));
    match saved {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    assert_eq!(result, Some("/home/alice/foo".to_owned()));
}

#[test]
fn compress_home_home_unset_no_substitution() {
    let _guard = crate::creds::HOME_MUTEX.lock().unwrap();
    let saved = std::env::var("HOME").ok();
    unsafe { std::env::remove_var("HOME") };
    let result = compress_home(std::path::Path::new("/home/alice/foo"));
    match saved {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    assert_eq!(result, Some("/home/alice/foo".to_owned()));
}

#[test]
fn compress_home_empty_path_returns_none() {
    let result = compress_home(std::path::Path::new(""));
    assert_eq!(result, None);
}

// ── project_dir ───────────────────────────────────────────────────────────

#[test]
fn project_dir_present_no_tilde() {
    let ctx = RenderCtx {
        project_dir: Some(PathBuf::from("/opt/project")),
        ..Default::default()
    };
    let result = render_placeholder("project_dir", &ctx);
    assert!(result.is_some());
    assert!(!result.unwrap().is_empty());
}

#[test]
fn project_dir_with_home_tilde() {
    let _guard = crate::creds::HOME_MUTEX.lock().unwrap();
    let saved = std::env::var("HOME").ok();
    unsafe { std::env::set_var("HOME", "/home/dev") };
    let ctx = RenderCtx {
        project_dir: Some(PathBuf::from("/home/dev/work/myproject")),
        ..Default::default()
    };
    let result = render_placeholder("project_dir", &ctx);
    match saved {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    assert_eq!(result, Some("~/work/myproject".to_owned()));
}

#[test]
fn project_dir_empty_path_returns_none() {
    let ctx = RenderCtx {
        project_dir: Some(PathBuf::from("")),
        ..Default::default()
    };
    assert_eq!(render_placeholder("project_dir", &ctx), None);
}

#[test]
fn project_dir_absent() {
    assert_eq!(render_placeholder("project_dir", &ctx_empty()), None);
}

// ── added_dirs_count ──────────────────────────────────────────────────────

#[test]
fn added_dirs_count_present() {
    let ctx = RenderCtx {
        added_dirs_count: Some(3),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("added_dirs_count", &ctx),
        Some("3".to_owned())
    );
}

#[test]
fn added_dirs_count_zero() {
    let ctx = RenderCtx {
        added_dirs_count: Some(0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("added_dirs_count", &ctx),
        Some("0".to_owned())
    );
}

#[test]
fn added_dirs_count_absent() {
    assert_eq!(render_placeholder("added_dirs_count", &ctx_empty()), None);
}

// ── workspace_git_worktree ────────────────────────────────────────────────
// Source: payload.workspace.git_worktree — present for ANY git worktree.

#[test]
fn workspace_git_worktree_present() {
    let ctx = RenderCtx {
        workspace_git_worktree: Some("feature-branch".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("workspace_git_worktree", &ctx),
        Some("feature-branch".to_owned())
    );
}

#[test]
fn workspace_git_worktree_absent() {
    assert_eq!(
        render_placeholder("workspace_git_worktree", &ctx_empty()),
        None
    );
}

// ── worktree_name ─────────────────────────────────────────────────────────
// Source: payload.worktree.name — only present during --worktree sessions.

#[test]
fn worktree_name_present() {
    let ctx = RenderCtx {
        worktree_name: Some("my-worktree".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("worktree_name", &ctx),
        Some("my-worktree".to_owned())
    );
}

#[test]
fn worktree_name_absent() {
    assert_eq!(render_placeholder("worktree_name", &ctx_empty()), None);
}

// ── worktree_path ─────────────────────────────────────────────────────────

#[test]
fn worktree_path_with_home_tilde() {
    let _guard = crate::creds::HOME_MUTEX.lock().unwrap();
    let saved = std::env::var("HOME").ok();
    unsafe { std::env::set_var("HOME", "/home/dev") };
    let ctx = RenderCtx {
        worktree_path: Some(PathBuf::from("/home/dev/trees/feature")),
        ..Default::default()
    };
    let result = render_placeholder("worktree_path", &ctx);
    match saved {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    assert_eq!(result, Some("~/trees/feature".to_owned()));
}

#[test]
fn worktree_path_empty_returns_none() {
    let ctx = RenderCtx {
        worktree_path: Some(PathBuf::from("")),
        ..Default::default()
    };
    assert_eq!(render_placeholder("worktree_path", &ctx), None);
}

#[test]
fn worktree_path_absent() {
    assert_eq!(render_placeholder("worktree_path", &ctx_empty()), None);
}

// ── worktree_branch ───────────────────────────────────────────────────────

#[test]
fn worktree_branch_present() {
    let ctx = RenderCtx {
        worktree_branch: Some("feat/new-thing".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("worktree_branch", &ctx),
        Some("feat/new-thing".to_owned())
    );
}

#[test]
fn worktree_branch_absent() {
    assert_eq!(render_placeholder("worktree_branch", &ctx_empty()), None);
}

// ── worktree_original_cwd ─────────────────────────────────────────────────

#[test]
fn worktree_original_cwd_with_home_tilde() {
    let _guard = crate::creds::HOME_MUTEX.lock().unwrap();
    let saved = std::env::var("HOME").ok();
    unsafe { std::env::set_var("HOME", "/home/dev") };
    let ctx = RenderCtx {
        worktree_original_cwd: Some(PathBuf::from("/home/dev/main-project")),
        ..Default::default()
    };
    let result = render_placeholder("worktree_original_cwd", &ctx);
    match saved {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    assert_eq!(result, Some("~/main-project".to_owned()));
}

#[test]
fn worktree_original_cwd_empty_returns_none() {
    let ctx = RenderCtx {
        worktree_original_cwd: Some(PathBuf::from("")),
        ..Default::default()
    };
    assert_eq!(render_placeholder("worktree_original_cwd", &ctx), None);
}

#[test]
fn worktree_original_cwd_absent() {
    assert_eq!(
        render_placeholder("worktree_original_cwd", &ctx_empty()),
        None
    );
}

// ── worktree_original_branch ──────────────────────────────────────────────

#[test]
fn worktree_original_branch_present() {
    let ctx = RenderCtx {
        worktree_original_branch: Some("main".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("worktree_original_branch", &ctx),
        Some("main".to_owned())
    );
}

#[test]
fn worktree_original_branch_absent() {
    assert_eq!(
        render_placeholder("worktree_original_branch", &ctx_empty()),
        None
    );
}

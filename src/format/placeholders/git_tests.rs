use super::*;
use std::path::PathBuf;

fn ctx_empty() -> RenderCtx {
    RenderCtx::default()
}

// ── git_branch ────────────────────────────────────────────────────────────

#[test]
fn git_branch_present() {
    let ctx = RenderCtx {
        git_branch: Some("main".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("git_branch", &ctx),
        Some("main".to_owned())
    );
}

#[test]
fn git_branch_absent() {
    assert_eq!(render_placeholder("git_branch", &ctx_empty()), None);
}

// ── git_root ──────────────────────────────────────────────────────────────

#[test]
fn git_root_present_no_home() {
    let ctx = RenderCtx {
        git_root: Some(PathBuf::from("/opt/project")),
        ..Default::default()
    };
    let result = render_placeholder("git_root", &ctx);
    assert!(result.is_some());
    assert!(!result.unwrap().is_empty());
}

#[test]
fn git_root_with_home_tilde() {
    let _guard = crate::creds::HOME_MUTEX.lock().unwrap();
    let saved = std::env::var("HOME").ok();
    unsafe { std::env::set_var("HOME", "/home/dev") };
    let ctx = RenderCtx {
        git_root: Some(PathBuf::from("/home/dev/projects/myrepo")),
        ..Default::default()
    };
    let result = render_placeholder("git_root", &ctx);
    match saved {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    assert_eq!(result, Some("~/projects/myrepo".to_owned()));
}

#[test]
fn git_root_empty_path_returns_none() {
    let ctx = RenderCtx {
        git_root: Some(PathBuf::from("")),
        ..Default::default()
    };
    assert_eq!(render_placeholder("git_root", &ctx), None);
}

#[test]
fn git_root_absent() {
    assert_eq!(render_placeholder("git_root", &ctx_empty()), None);
}

// ── git_changes ───────────────────────────────────────────────────────────

#[test]
fn git_changes_nonzero() {
    let ctx = RenderCtx {
        git_changes_count: Some(5),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("git_changes", &ctx),
        Some("5".to_owned())
    );
}

#[test]
fn git_changes_zero() {
    let ctx = RenderCtx {
        git_changes_count: Some(0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("git_changes", &ctx),
        Some("0".to_owned())
    );
}

#[test]
fn git_changes_absent() {
    assert_eq!(render_placeholder("git_changes", &ctx_empty()), None);
}

// ── git_staged ────────────────────────────────────────────────────────────

#[test]
fn git_staged_present() {
    let ctx = RenderCtx {
        git_staged_count: Some(2),
        ..Default::default()
    };
    assert_eq!(render_placeholder("git_staged", &ctx), Some("2".to_owned()));
}

#[test]
fn git_staged_absent() {
    assert_eq!(render_placeholder("git_staged", &ctx_empty()), None);
}

// ── git_unstaged ──────────────────────────────────────────────────────────

#[test]
fn git_unstaged_present() {
    let ctx = RenderCtx {
        git_unstaged_count: Some(3),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("git_unstaged", &ctx),
        Some("3".to_owned())
    );
}

#[test]
fn git_unstaged_absent() {
    assert_eq!(render_placeholder("git_unstaged", &ctx_empty()), None);
}

// ── git_untracked ─────────────────────────────────────────────────────────

#[test]
fn git_untracked_present() {
    let ctx = RenderCtx {
        git_untracked_count: Some(1),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("git_untracked", &ctx),
        Some("1".to_owned())
    );
}

#[test]
fn git_untracked_absent() {
    assert_eq!(render_placeholder("git_untracked", &ctx_empty()), None);
}

// ── git_status_clean ──────────────────────────────────────────────────────

#[test]
fn git_status_clean_all_zero_returns_clean() {
    let ctx = RenderCtx {
        git_changes_count: Some(0),
        git_staged_count: Some(0),
        git_unstaged_count: Some(0),
        git_untracked_count: Some(0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("git_status_clean", &ctx),
        Some("clean".to_owned())
    );
}

#[test]
fn git_status_clean_with_changes_returns_none() {
    let ctx = RenderCtx {
        git_changes_count: Some(1),
        git_staged_count: Some(1),
        git_unstaged_count: Some(0),
        git_untracked_count: Some(0),
        ..Default::default()
    };
    assert_eq!(render_placeholder("git_status_clean", &ctx), None);
}

#[test]
fn git_status_clean_with_untracked_returns_none() {
    let ctx = RenderCtx {
        git_changes_count: Some(1),
        git_staged_count: Some(0),
        git_unstaged_count: Some(0),
        git_untracked_count: Some(1),
        ..Default::default()
    };
    assert_eq!(render_placeholder("git_status_clean", &ctx), None);
}

#[test]
fn git_status_clean_partial_none_counts_returns_none() {
    // If counts are absent (not in a repo), git_status_clean returns None.
    let ctx = RenderCtx {
        git_changes_count: None,
        git_staged_count: None,
        git_unstaged_count: None,
        git_untracked_count: None,
        ..Default::default()
    };
    assert_eq!(render_placeholder("git_status_clean", &ctx), None);
}

#[test]
fn git_status_clean_missing_one_count_returns_none() {
    // Three zeros but one None → not "clean".
    let ctx = RenderCtx {
        git_changes_count: Some(0),
        git_staged_count: Some(0),
        git_unstaged_count: None,
        git_untracked_count: Some(0),
        ..Default::default()
    };
    assert_eq!(render_placeholder("git_status_clean", &ctx), None);
}

// ── git_root HOME compression edge cases ─────────────────────────────────

#[test]
fn git_root_exact_home_path_compresses_to_tilde() {
    let _guard = crate::creds::HOME_MUTEX.lock().unwrap();
    let saved = std::env::var("HOME").ok();
    unsafe { std::env::set_var("HOME", "/home/dev") };
    let ctx = RenderCtx {
        git_root: Some(std::path::PathBuf::from("/home/dev")),
        ..Default::default()
    };
    let result = render_placeholder("git_root", &ctx);
    match saved {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    // /home/dev itself → "~" (tilde with empty suffix).
    assert_eq!(result, Some("~".to_owned()));
}

#[test]
fn git_root_path_not_under_home_is_unchanged() {
    let _guard = crate::creds::HOME_MUTEX.lock().unwrap();
    let saved = std::env::var("HOME").ok();
    unsafe { std::env::set_var("HOME", "/home/dev") };
    let ctx = RenderCtx {
        git_root: Some(std::path::PathBuf::from("/opt/other/project")),
        ..Default::default()
    };
    let result = render_placeholder("git_root", &ctx);
    match saved {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    assert_eq!(result, Some("/opt/other/project".to_owned()));
}

#[test]
fn git_root_unicode_path_roundtrips() {
    let ctx = RenderCtx {
        git_root: Some(std::path::PathBuf::from("/opt/répertoire/projet")),
        ..Default::default()
    };
    let result = render_placeholder("git_root", &ctx);
    assert_eq!(result, Some("/opt/répertoire/projet".to_owned()));
}

// ── case-sensitivity and unknown git-prefixed names ───────────────────────

#[test]
fn git_branch_mixed_case_name_is_unknown_placeholder() {
    // `gIt_branch` (mixed case) must NOT match the git_branch arm — returns None.
    assert_eq!(render_placeholder("gIt_branch", &ctx_empty()), None);
}

#[test]
fn git_underscore_only_is_unknown_placeholder() {
    // `git_` with no suffix is not a registered placeholder.
    assert_eq!(render_placeholder("git_", &ctx_empty()), None);
}

#[test]
fn git_branch_empty_string_branch_is_some() {
    // An empty branch string is unusual but the RenderCtx field can hold it.
    // The placeholder returns Some("") — renderer decides whether to show it.
    let ctx = RenderCtx {
        git_branch: Some(String::new()),
        ..Default::default()
    };
    assert_eq!(render_placeholder("git_branch", &ctx), Some(String::new()));
}

// ── git_changes zero value is rendered (not None) ────────────────────────

#[test]
fn git_staged_zero_renders_as_zero_string() {
    let ctx = RenderCtx {
        git_staged_count: Some(0),
        ..Default::default()
    };
    assert_eq!(render_placeholder("git_staged", &ctx), Some("0".to_owned()));
}

#[test]
fn git_unstaged_zero_renders_as_zero_string() {
    let ctx = RenderCtx {
        git_unstaged_count: Some(0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("git_unstaged", &ctx),
        Some("0".to_owned())
    );
}

#[test]
fn git_untracked_zero_renders_as_zero_string() {
    let ctx = RenderCtx {
        git_untracked_count: Some(0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("git_untracked", &ctx),
        Some("0".to_owned())
    );
}

// ── git_status_clean: unstaged-only dirty still returns None ─────────────

#[test]
fn git_status_clean_only_unstaged_nonzero_returns_none() {
    let ctx = RenderCtx {
        git_changes_count: Some(1),
        git_staged_count: Some(0),
        git_unstaged_count: Some(1),
        git_untracked_count: Some(0),
        ..Default::default()
    };
    assert_eq!(render_placeholder("git_status_clean", &ctx), None);
}

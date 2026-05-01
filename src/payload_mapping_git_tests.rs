use super::*;

/// Initialize a bare-minimum git repo via shell-out.
fn init_git_repo_with_commit(dir: &std::path::Path) {
    use std::process::Command;
    Command::new("git")
        .args(["-C", dir.to_str().unwrap(), "init", "-b", "main"])
        .status()
        .expect("git init");
    let f = dir.join("f.txt");
    std::fs::write(&f, b"hello").expect("write");
    Command::new("git")
        .args(["-C", dir.to_str().unwrap(), "add", "."])
        .status()
        .expect("git add");
    Command::new("git")
        .args([
            "-C",
            dir.to_str().unwrap(),
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

#[test]
fn populate_git_ctx_inside_repo_sets_branch() {
    let dir = tempfile::tempdir().expect("tempdir");
    init_git_repo_with_commit(dir.path());
    let mut ctx = crate::format::RenderCtx::default();
    populate_git_ctx(&mut ctx, dir.path());
    assert_eq!(ctx.git_branch.as_deref(), Some("main"));
    assert!(ctx.git_root.is_some());
    // Status counts must also be populated.
    assert!(ctx.git_changes_count.is_some());
    assert!(ctx.git_staged_count.is_some());
    assert!(ctx.git_unstaged_count.is_some());
    assert!(ctx.git_untracked_count.is_some());
}

#[test]
fn populate_git_ctx_outside_repo_all_none() {
    let _guard = crate::git::GIT_ENV_MUTEX.lock().unwrap();
    let dir = tempfile::tempdir().expect("tempdir");
    std::env::set_var("GIT_CEILING_DIRECTORIES", dir.path());
    let mut ctx = crate::format::RenderCtx::default();
    populate_git_ctx(&mut ctx, dir.path());
    std::env::remove_var("GIT_CEILING_DIRECTORIES");
    assert!(ctx.git_branch.is_none());
    assert!(ctx.git_root.is_none());
    assert!(ctx.git_changes_count.is_none());
    assert!(ctx.git_staged_count.is_none());
    assert!(ctx.git_unstaged_count.is_none());
    assert!(ctx.git_untracked_count.is_none());
}

#[test]
fn populate_git_ctx_nonexistent_cwd_returns_all_none() {
    let _guard = crate::git::GIT_ENV_MUTEX.lock().unwrap();
    let dir = tempfile::tempdir().expect("tempdir");
    let gone = dir.path().join("does_not_exist_xyz");
    std::env::set_var("GIT_CEILING_DIRECTORIES", dir.path());
    let mut ctx = crate::format::RenderCtx::default();
    populate_git_ctx(&mut ctx, &gone);
    std::env::remove_var("GIT_CEILING_DIRECTORIES");
    assert!(
        ctx.git_branch.is_none(),
        "nonexistent cwd must yield None branch"
    );
    assert!(
        ctx.git_root.is_none(),
        "nonexistent cwd must yield None root"
    );
}

#[test]
fn populate_git_ctx_subdir_discovers_repo() {
    let dir = tempfile::tempdir().expect("tempdir");
    init_git_repo_with_commit(dir.path());
    // Create a subdirectory inside the repo.
    let subdir = dir.path().join("deep").join("nested");
    std::fs::create_dir_all(&subdir).expect("mkdir");
    let mut ctx = crate::format::RenderCtx::default();
    populate_git_ctx(&mut ctx, &subdir);
    // gix walks parent directories, so branch must still be found.
    assert_eq!(
        ctx.git_branch.as_deref(),
        Some("main"),
        "gix must discover repo from subdirectory"
    );
    assert!(ctx.git_root.is_some());
}

#[test]
fn populate_git_ctx_path_with_spaces_and_unicode() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fancy = dir.path().join("my repo ñ");
    std::fs::create_dir_all(&fancy).expect("mkdir unicode");
    init_git_repo_with_commit(&fancy);
    let mut ctx = crate::format::RenderCtx::default();
    populate_git_ctx(&mut ctx, &fancy);
    assert_eq!(
        ctx.git_branch.as_deref(),
        Some("main"),
        "paths with spaces/unicode must work"
    );
}

#[test]
fn populate_git_ctx_detached_head_branch_is_none() {
    use std::process::Command;
    let dir = tempfile::tempdir().expect("tempdir");
    init_git_repo_with_commit(dir.path());
    // Detach HEAD.
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
    let mut ctx = crate::format::RenderCtx::default();
    populate_git_ctx(&mut ctx, dir.path());
    assert!(
        ctx.git_branch.is_none(),
        "detached HEAD must yield None branch"
    );
    // root and counts must still be populated.
    assert!(
        ctx.git_root.is_some(),
        "root must be Some even in detached HEAD"
    );
    assert!(ctx.git_staged_count.is_some());
}

#[test]
fn populate_git_ctx_does_not_overwrite_pre_existing_non_git_fields() {
    let dir = tempfile::tempdir().expect("tempdir");
    init_git_repo_with_commit(dir.path());
    let mut ctx = crate::format::RenderCtx {
        model: Some("claude-3".to_owned()),
        five_used: Some(42.0),
        ..Default::default()
    };
    populate_git_ctx(&mut ctx, dir.path());
    assert_eq!(
        ctx.model.as_deref(),
        Some("claude-3"),
        "populate_git_ctx must not clobber unrelated fields"
    );
    assert_eq!(ctx.five_used, Some(42.0));
}

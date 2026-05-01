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

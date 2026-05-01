//! Phase-2 golden tests: placeholder expansion + git module integration.
//!
//! Spawns the release binary with the full-payload fixture and asserts
//! that all new Phase-2 placeholders render to their expected values.

use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use tempfile::TempDir;

// ── helpers ───────────────────────────────────────────────────────────────────

fn bin() -> Command {
    Command::cargo_bin("cc-myasl").expect("binary must build")
}

fn full_payload() -> String {
    fs::read_to_string("tests/fixtures/full-payload.json").expect("full-payload.json must exist")
}

fn write_config(dir: &TempDir, json: &str) -> PathBuf {
    let path = dir.path().join("cfg.json");
    fs::write(&path, json).unwrap();
    path
}

/// Build a config JSON with a single-line, single-segment template.
fn single_segment_config(template: &str) -> String {
    let escaped = template.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        r#"{{"lines":[{{"separator":"","segments":[{{"template":"{escaped}","hide_when_absent":true}}]}}]}}"#
    )
}

/// Build a config JSON with multiple segments (each hide_when_absent) on one line.
fn multi_segment_config(templates: &[&str]) -> String {
    let segs: Vec<String> = templates
        .iter()
        .map(|t| {
            let escaped = t.replace('\\', "\\\\").replace('"', "\\\"");
            format!(r#"{{"template":"{escaped}","hide_when_absent":true}}"#)
        })
        .collect();
    format!(
        r#"{{"lines":[{{"separator":"","segments":[{}]}}]}}"#,
        segs.join(",")
    )
}

// ── helper: create a minimal git repo with one commit ────────────────────────

fn init_repo_with_commit(branch: &str) -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = dir.path().to_str().unwrap();

    let out = StdCommand::new("git")
        .args(["-C", p, "init", "-b", branch])
        .output()
        .expect("git init");
    if !out.status.success() {
        StdCommand::new("git")
            .args(["-C", p, "init"])
            .status()
            .expect("git init fallback");
        StdCommand::new("git")
            .args(["-C", p, "checkout", "-b", branch])
            .status()
            .expect("git checkout -b");
    }

    fs::write(dir.path().join("file.txt"), b"hello").unwrap();
    StdCommand::new("git")
        .args(["-C", p, "add", "."])
        .status()
        .expect("git add");
    StdCommand::new("git")
        .args([
            "-C",
            p,
            "-c",
            "user.email=t@t.com",
            "-c",
            "user.name=T",
            "commit",
            "-m",
            "initial",
        ])
        .status()
        .expect("git commit");

    dir
}

// ── test A: golden_full_payload_renders_all_placeholders ─────────────────────

/// Run the binary with a config referencing every new Phase-2 placeholder
/// and assert each one renders to its expected value from full-payload.json.
///
/// Note: {git_*} placeholders are exercised separately in tests C and D
/// because they depend on the binary's working directory being (or not being)
/// inside a git repo.
#[test]
fn golden_full_payload_renders_all_placeholders() {
    let home = tempfile::tempdir().unwrap();
    let cfg_dir = tempfile::tempdir().unwrap();

    // Each entry: (placeholder_template, expected_substring_in_output).
    // The segment is hide_when_absent so if it returns None the output is empty.
    let cases: &[(&str, &str)] = &[
        ("{model_id}", "claude-sonnet-4-6"),
        ("{version}", "1.2.3"),
        ("{session_id}", "session-abc123"),
        ("{session_name}", "My Project Session"),
        ("{output_style}", "verbose"),
        ("{effort}", "high"),
        ("{thinking_enabled}", "thinking"),
        ("{vim_mode}", "normal"),
        ("{agent_name}", "code-agent"),
        ("{cost_usd}", "0.42"),
        ("{session_clock}", "2h3m"),
        ("{api_duration}", "1h55m"),
        ("{lines_added}", "150"),
        ("{lines_removed}", "30"),
        ("{lines_changed}", "180"),
        ("{tokens_input}", "4.1k"),
        ("{tokens_output}", "512"),
        ("{tokens_cached_creation}", "1.0k"),
        ("{tokens_cached_read}", "2.0k"),
        ("{tokens_cached_total}", "3.1k"),
        ("{tokens_total}", "7.7k"),
        ("{tokens_input_total}", "12.3k"),
        ("{tokens_output_total}", "6.8k"),
        ("{context_size}", "200000"),
        ("{context_used_pct}", "23.5"),
        ("{context_remaining_pct}", "76.5"),
        ("{context_used_pct_int}", "23"),
        ("{context_bar}", "[██░░░░░░░░]"),
        ("{added_dirs_count}", "2"),
        ("{workspace_git_worktree}", "feature-branch"),
        ("{worktree_name}", "feature-branch"),
        ("{worktree_branch}", "feature/add-placeholders"),
        ("{worktree_original_branch}", "main"),
    ];

    for (tmpl, expected) in cases {
        let cfg_json = single_segment_config(tmpl);
        let cfg = write_config(&cfg_dir, &cfg_json);

        let out = bin()
            .arg("--config")
            .arg(&cfg)
            .env("HOME", home.path())
            .env("XDG_CACHE_HOME", home.path().join(".cache"))
            .env("XDG_CONFIG_HOME", home.path().join(".config"))
            .env("STATUSLINE_OAUTH_BASE_URL", "http://127.0.0.1:1")
            .write_stdin(full_payload())
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let out_str = String::from_utf8(out).unwrap();
        assert!(
            out_str.contains(expected),
            "placeholder {tmpl}: expected {expected:?} in output; got: {out_str:?}"
        );
    }
}

// ── test B: golden_rich_template_smoke ────────────────────────────────────────

/// Run with `--template rich` against the full-payload fixture.
/// Assert: non-empty output and the model name appears on line 0.
/// (git placeholders on line 1 may or may not resolve depending on cwd;
/// cost/clock/tokens on line 2 always resolve from the fixture.)
#[test]
fn golden_rich_template_smoke() {
    let home = tempfile::tempdir().unwrap();

    let out = bin()
        .arg("--template")
        .arg("rich")
        .env("HOME", home.path())
        .env("XDG_CACHE_HOME", home.path().join(".cache"))
        .env("XDG_CONFIG_HOME", home.path().join(".config"))
        .env("STATUSLINE_OAUTH_BASE_URL", "http://127.0.0.1:1")
        .write_stdin(full_payload())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let out_str = String::from_utf8(out).unwrap();
    assert!(
        !out_str.trim().is_empty(),
        "rich template must produce non-empty output"
    );

    let lines: Vec<&str> = out_str.trim_end_matches('\n').split('\n').collect();
    assert!(!lines.is_empty(), "must have at least one line");

    // Line 0 contains model + vim_mode (fixture has "normal") + context bar.
    assert!(
        lines[0].contains("claude-sonnet-4-6"),
        "line 0 must contain model display name; got: {:?}",
        lines[0]
    );
    assert!(
        lines[0].contains("normal"),
        "line 0 must contain vim_mode; got: {:?}",
        lines[0]
    );
    assert!(
        lines[0].contains("ctx:"),
        "line 0 must contain ctx: prefix; got: {:?}",
        lines[0]
    );

    // Line 2 (cost/clock/tokens) — always resolves from the fixture.
    if lines.len() >= 3 {
        let cost_line = lines[2];
        assert!(
            cost_line.contains("0.42$"),
            "cost line must contain '0.42$'; got: {:?}",
            cost_line
        );
        assert!(
            cost_line.contains("2h3m"),
            "cost line must contain session_clock '2h3m'; got: {:?}",
            cost_line
        );
    }
}

// ── test C: golden_git_placeholders_outside_repo_collapse ────────────────────

/// Set the binary's effective cwd (via GIT_CEILING_DIRECTORIES) to a tempdir
/// that is not inside a git repo.  Assert that {git_branch} and friends
/// produce no output (segments collapse via hide_when_absent).
#[test]
fn golden_git_placeholders_outside_repo_collapse() {
    let no_repo_dir = tempfile::tempdir().unwrap();
    let cfg_dir = tempfile::tempdir().unwrap();

    // Config: two hide_when_absent segments using git_* placeholders.
    let cfg_json = multi_segment_config(&["{git_branch}", "{git_changes}"]);
    let cfg = write_config(&cfg_dir, &cfg_json);

    // We cannot change the binary's cwd via assert_cmd, but we can prevent
    // gix from walking above the tempdir by setting GIT_CEILING_DIRECTORIES.
    // The binary resolves cwd from payload.workspace.current_dir ("/Users/test/myproject"),
    // which is not a real path; gix discover will fail there too.
    // For belt-and-suspenders: we also override HOME to an isolated tempdir.
    let home = tempfile::tempdir().unwrap();

    let out = bin()
        .arg("--config")
        .arg(&cfg)
        .env("HOME", home.path())
        .env("XDG_CACHE_HOME", home.path().join(".cache"))
        .env("XDG_CONFIG_HOME", home.path().join(".config"))
        // Ceiling stops gix walking above the no-repo tempdir.
        .env("GIT_CEILING_DIRECTORIES", no_repo_dir.path())
        .env("STATUSLINE_OAUTH_BASE_URL", "http://127.0.0.1:1")
        .write_stdin(full_payload())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let out_str = String::from_utf8(out).unwrap();
    // Both segments are hide_when_absent; if both collapse the line is empty.
    // Output must NOT contain the literal placeholder text.
    assert!(
        !out_str.contains("{git_branch}"),
        "output must not contain un-rendered literal {{git_branch}}; got: {:?}",
        out_str
    );
    assert!(
        !out_str.contains("{git_changes}"),
        "output must not contain un-rendered literal {{git_changes}}; got: {:?}",
        out_str
    );
    // The payload's cwd /Users/test/myproject is not a real repo so git_branch is None.
    // Both segments collapse → the line (possibly the entire output) is empty or whitespace.
    let trimmed = out_str.trim();
    assert!(
        trimmed.is_empty(),
        "all git segments must collapse; got: {:?}",
        trimmed
    );
}

// ── test D: golden_git_placeholders_inside_repo_resolve ──────────────────────

/// Create a real git repo (tempdir + git init + commit), pipe the full-payload
/// fixture with a modified cwd pointing into that repo, and assert that
/// {git_branch} resolves to "main" and {git_root} resolves to the repo path.
#[test]
fn golden_git_placeholders_inside_repo_resolve() {
    let repo_dir = init_repo_with_commit("main");
    let repo_path = repo_dir.path().to_str().unwrap().to_owned();
    let cfg_dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // Craft a payload that points cwd at the real repo.
    let payload_with_repo_cwd = format!(
        r#"{{
  "model": {{"id": "claude-sonnet-4-6", "display_name": "claude-sonnet-4-6"}},
  "cwd": "{repo_path}",
  "workspace": {{"current_dir": "{repo_path}"}},
  "rate_limits": {{
    "five_hour": {{"used_percentage": 10.0, "resets_at": 9999999999}},
    "seven_day": {{"used_percentage": 20.0, "resets_at": 9999999999}}
  }}
}}"#
    );

    // Config with git_branch and git_root, both hide_when_absent.
    let cfg_json = multi_segment_config(&["{git_branch}", "|{git_root}"]);
    let cfg = write_config(&cfg_dir, &cfg_json);

    let out = bin()
        .arg("--config")
        .arg(&cfg)
        .env("HOME", home.path())
        .env("XDG_CACHE_HOME", home.path().join(".cache"))
        .env("XDG_CONFIG_HOME", home.path().join(".config"))
        .env("STATUSLINE_OAUTH_BASE_URL", "http://127.0.0.1:1")
        .write_stdin(payload_with_repo_cwd)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let out_str = String::from_utf8(out).unwrap();
    assert!(
        out_str.contains("main"),
        "git_branch must resolve to 'main'; got: {:?}",
        out_str
    );
    // git_root is home-compressed when under HOME; here HOME is a tempdir,
    // so git_root will NOT be under HOME and renders as the full path.
    assert!(
        out_str.contains(&repo_path) || out_str.contains("~"),
        "git_root must contain repo path; got: {:?}",
        out_str
    );
}

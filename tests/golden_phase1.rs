//! Phase-1 golden tests: migration safety net + new layout features.
//!
//! Spawns the release binary against controlled fixtures and config files.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;

// ── helpers ───────────────────────────────────────────────────────────────────

fn bin() -> Command {
    Command::cargo_bin("cc-myasl").expect("binary must build")
}

fn write_config(dir: &tempfile::TempDir, json: &str) -> PathBuf {
    let path = dir.path().join("cfg.json");
    fs::write(&path, json).unwrap();
    path
}

// ── snapshot constants (inlined from tests/snapshots/builtin-outputs.txt) ────
//
// Captured at Task 0 baseline before any structured-config changes.
// Fixture: FIXTURE_RESETS_AT_1 below, rendered with TZ=UTC.
//   five_hour.used_percentage=24.0 → five_left=76
//   seven_day.used_percentage=41.0 → seven_left=59
//   resets_at=1 → "00:00" UTC

const SNAPSHOT_DEFAULT: &str = "claude-opus-4-7 \u{00b7} 5h: 76% \u{00b7} 7d: 59% (resets 00:00)";
const SNAPSHOT_MINIMAL: &str = "claude-opus-4-7 76%/59%";
const SNAPSHOT_COMPACT: &str = "claude-opus-4-7 76/59";
const SNAPSHOT_BARS: &str = concat!(
    "claude-opus-4-7 5h:[\u{2588}\u{2588}\u{2588}\u{2588}",
    "\u{2588}\u{2588}\u{2588}\u{2588}\u{2591}\u{2591}] 7d:[",
    "\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}",
    "\u{2591}\u{2591}\u{2591}\u{2591}]"
);
const SNAPSHOT_COLORED: &str = concat!(
    "claude-opus-4-7 \u{00b7} 5h: \x1b[32m76%\x1b[0m",
    " \u{00b7} 7d: \x1b[32m59%\x1b[0m"
);
const SNAPSHOT_EMOJI: &str = concat!(
    "claude-opus-4-7 \u{00b7} \u{1F7E2} 5h 76%",
    " \u{00b7} \u{1F7E2} 7d 59%"
);
const SNAPSHOT_EMOJI_VERBOSE: &str = concat!(
    "\u{1F916} claude-opus-4-7 \u{00b7} \u{1F7E2} proj",
    " \u{00b7} \u{23F3} 76%/59% \u{00b7} \u{23F0} 00:00"
);
const SNAPSHOT_VERBOSE: &str = concat!(
    "claude-opus-4-7 \u{00b7} proj \u{00b7} 5h:[",
    "\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}",
    "\u{2591}\u{2591}] 76% (in 0m) \u{00b7} 7d:[",
    "\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}",
    "\u{2591}\u{2591}\u{2591}\u{2591}] 59% (in 0m)"
);

/// Fixture with resets_at=1 → renders as "00:00" in UTC.
const FIXTURE_RESETS_AT_1: &str = r#"{
  "model": { "display_name": "claude-opus-4-7" },
  "workspace": { "current_dir": "/Users/test/proj" },
  "transcript_path": "/tmp/t.jsonl",
  "session_id": "fixture-pro-max",
  "rate_limits": {
    "five_hour": { "used_percentage": 24.0, "resets_at": 1 },
    "seven_day": { "used_percentage": 41.0, "resets_at": 1 }
  }
}"#;

// ── test 9: golden_output_unchanged — byte-exact migration safety net ─────────

/// For each of 8 built-in template names, render via the release binary and
/// assert byte-exact match against the Task 0 snapshot.
/// If this fails for any built-in, the struct-literal migration diverged from
/// the .txt original — stop, inspect the diff, and fix the struct literal.
#[test]
fn golden_output_unchanged() {
    let cases: &[(&str, &str)] = &[
        ("default", SNAPSHOT_DEFAULT),
        ("minimal", SNAPSHOT_MINIMAL),
        ("compact", SNAPSHOT_COMPACT),
        ("bars", SNAPSHOT_BARS),
        ("colored", SNAPSHOT_COLORED),
        ("emoji", SNAPSHOT_EMOJI),
        ("emoji_verbose", SNAPSHOT_EMOJI_VERBOSE),
        ("verbose", SNAPSHOT_VERBOSE),
    ];

    for (name, expected) in cases {
        let out = bin()
            .arg("--template")
            .arg(name)
            .env("TZ", "UTC")
            .env("STATUSLINE_OAUTH_BASE_URL", "http://127.0.0.1:1")
            .write_stdin(FIXTURE_RESETS_AT_1)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let actual = String::from_utf8(out).unwrap();
        let expected_with_newline = format!("{}\n", expected);
        assert_eq!(
            actual, expected_with_newline,
            "template '{}' output diverged from snapshot",
            name
        );
    }
}

// ── test 10: golden_multiline_output ─────────────────────────────────────────

/// A 2-line config produces output with exactly one newline separating the lines.
#[test]
fn golden_multiline_output() {
    let tmpdir = tempfile::tempdir().unwrap();
    let cfg = write_config(
        &tmpdir,
        r#"{
          "lines": [
            {"separator": "", "segments": [{"template": "{model}"}]},
            {"separator": "", "segments": [{"template": "{five_left}%/{seven_left}%"}]}
          ]
        }"#,
    );

    let out = bin()
        .arg("--config")
        .arg(&cfg)
        .env("STATUSLINE_OAUTH_BASE_URL", "http://127.0.0.1:1")
        .write_stdin(FIXTURE_RESETS_AT_1)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let out_str = String::from_utf8(out).unwrap();
    let trimmed = out_str.trim_end_matches('\n');
    let newline_count = trimmed.matches('\n').count();
    assert_eq!(
        newline_count, 1,
        "expected exactly one \\n; got: {out_str:?}"
    );

    let parts: Vec<&str> = trimmed.split('\n').collect();
    assert_eq!(parts.len(), 2);
    assert!(parts[0].contains("claude-opus-4-7"), "line 0: {}", parts[0]);
    assert!(
        parts[1].contains("76%") && parts[1].contains("59%"),
        "line 1: {}",
        parts[1]
    );
}

// ── test 11: golden_flex_spacer ───────────────────────────────────────────────

/// A flex spacer produces at least one space between left and right segments.
#[test]
fn golden_flex_spacer() {
    let tmpdir = tempfile::tempdir().unwrap();
    let cfg = write_config(
        &tmpdir,
        r#"{
          "lines": [{
            "separator": "",
            "segments": [
              {"template": "LEFT"},
              {"flex": true},
              {"template": "RIGHT"}
            ]
          }]
        }"#,
    );

    let out = bin()
        .arg("--config")
        .arg(&cfg)
        .env("STATUSLINE_OAUTH_BASE_URL", "http://127.0.0.1:1")
        .write_stdin(FIXTURE_RESETS_AT_1)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let out_str = String::from_utf8(out).unwrap();
    let line = out_str.trim_end_matches('\n');
    assert!(
        line.starts_with("LEFT"),
        "must start with LEFT; got: {line:?}"
    );
    assert!(
        line.ends_with("RIGHT"),
        "must end with RIGHT; got: {line:?}"
    );
    // The flex region must be ≥1 space.
    let between = &line["LEFT".len()..line.len() - "RIGHT".len()];
    assert!(
        !between.is_empty() && between.chars().all(|c| c == ' '),
        "flex region must be ≥1 space; got: {between:?}"
    );
}

// ── test 12: golden_user_template_shadows_builtin ────────────────────────────

/// A user template at `<XDG_CONFIG_HOME>/cc-myasl/templates/default.json`
/// shadows the built-in default when `--template default` is invoked.
#[test]
fn golden_user_template_shadows_builtin() {
    let tmpdir = tempfile::tempdir().unwrap();
    let config_home = tmpdir.path().to_path_buf();

    let templates_dir = config_home.join("cc-myasl").join("templates");
    fs::create_dir_all(&templates_dir).unwrap();

    let sentinel = "SENTINEL_XYZ_unique";
    fs::write(
        templates_dir.join("default.json"),
        format!(r#"{{"lines":[{{"separator":"","segments":[{{"template":"{sentinel}"}}]}}]}}"#),
    )
    .unwrap();

    bin()
        .arg("--template")
        .arg("default")
        .env("XDG_CONFIG_HOME", &config_home)
        .env("STATUSLINE_OAUTH_BASE_URL", "http://127.0.0.1:1")
        .write_stdin(FIXTURE_RESETS_AT_1)
        .assert()
        .success()
        .stdout(predicates::str::contains(sentinel))
        .stdout(predicates::str::contains("claude-opus-4-7").not());
}

// ── test 13: golden_invalid_config_falls_back ─────────────────────────────────

/// A config with 4 lines (exceeds MAX_LINES=3) is rejected; the binary falls
/// back to the embedded default. Exit code must be 0.
#[test]
fn golden_invalid_config_falls_back() {
    let tmpdir = tempfile::tempdir().unwrap();
    let cfg = write_config(
        &tmpdir,
        r#"{
          "lines": [
            {"separator": "", "segments": [{"template": "L1"}]},
            {"separator": "", "segments": [{"template": "L2"}]},
            {"separator": "", "segments": [{"template": "L3"}]},
            {"separator": "", "segments": [{"template": "L4"}]}
          ]
        }"#,
    );

    bin()
        .arg("--config")
        .arg(&cfg)
        .env("STATUSLINE_OAUTH_BASE_URL", "http://127.0.0.1:1")
        .write_stdin(FIXTURE_RESETS_AT_1)
        .assert()
        .success()
        // Default template contains the model name — confirms fallback worked.
        .stdout(predicates::str::contains("claude-opus-4-7"));
}

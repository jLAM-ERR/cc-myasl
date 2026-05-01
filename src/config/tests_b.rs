/// Adversarial / boundary tests for config/mod.rs.
///
/// Companion to tests.rs (which covers happy paths and basic error paths).
/// Split here because tests.rs was already close to the 500-LOC ceiling.
use super::*;
use crate::args::Args;
use crate::debug::{ConfigSource, Trace};
use std::path::PathBuf;
use tempfile::tempdir;

// ── helpers ───────────────────────────────────────────────────────────────

fn empty_args() -> Args {
    Args::default()
}

fn lock_config() -> std::sync::MutexGuard<'static, ()> {
    CONFIG_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

// ── from_file: JSON type mismatches ──────────────────────────────────────

#[test]
fn from_file_null_json_returns_config_parse_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("null.json");
    std::fs::write(&path, b"null").unwrap();
    let err = from_file(&path).unwrap_err();
    assert!(
        matches!(err, crate::error::Error::ConfigParse(_)),
        "JSON null must yield ConfigParse, got {err:?}"
    );
}

#[test]
fn from_file_empty_object_succeeds_with_default_lines() {
    // {} deserialises via #[serde(default)] on Config: the missing `lines` field
    // fills with Config::default().lines (the embedded builtin), not an empty vec.
    // 0 <= MAX_LINES so validation passes regardless.
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty.json");
    std::fs::write(&path, b"{}").unwrap();
    let cfg = from_file(&path).expect("empty JSON object must parse and validate");
    // The result is valid — that's the key invariant. The exact line count is
    // whatever Config::default() contains; we don't hard-code it here.
    let mut cfg_clone = cfg.clone();
    assert!(
        cfg_clone.validate_and_clamp().is_ok(),
        "config from empty object must pass validation"
    );
}

// ── from_file: path is a directory ────────────────────────────────────────

#[test]
fn from_file_directory_path_returns_config_parse_error() {
    let dir = tempdir().unwrap();
    // Pass the directory itself as the path — read_to_string on a dir errors.
    let err = from_file(dir.path()).unwrap_err();
    assert!(
        matches!(err, crate::error::Error::ConfigParse(_)),
        "directory path must yield ConfigParse, got {err:?}"
    );
}

// ── from_file: UTF-8 BOM ──────────────────────────────────────────────────

#[test]
fn from_file_utf8_bom_prefix_returns_config_parse_error() {
    // serde_json does not strip the UTF-8 BOM (EF BB BF) before parsing,
    // so the document is not valid JSON from its perspective.
    let dir = tempdir().unwrap();
    let path = dir.path().join("bom.json");
    let bom_json = b"\xef\xbb\xbf{\"lines\":[]}";
    std::fs::write(&path, bom_json).unwrap();
    let result = from_file(&path);
    // serde_json treats BOM as an unexpected character → parse error.
    assert!(
        result.is_err(),
        "UTF-8 BOM-prefixed JSON should not parse successfully"
    );
    if let Err(e) = result {
        assert!(
            matches!(e, crate::error::Error::ConfigParse(_)),
            "BOM parse failure must yield ConfigParse, got {e:?}"
        );
    }
}

// ── from_file: permission denied (unix only) ──────────────────────────────

#[test]
#[cfg(unix)]
fn from_file_permission_denied_returns_config_parse_error() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempdir().unwrap();
    let path = dir.path().join("noperm.json");
    std::fs::write(&path, b"{\"lines\":[]}").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    let err = from_file(&path).unwrap_err();
    // Restore so tempdir cleanup can delete it.
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(
        matches!(err, crate::error::Error::ConfigParse(_)),
        "permission denied must yield ConfigParse, got {err:?}"
    );
}

// ── user_template_path: name with extension is rejected ──────────────────

#[test]
fn user_template_path_name_with_dot_is_rejected() {
    // "compact.json" contains a dot — rejected by the safe-name check.
    // Users must pass "compact" not "compact.json".
    let config_dir = PathBuf::from("/home/user/.config/cc-myasl");
    assert!(
        user_template_path(&config_dir, "compact.json").is_none(),
        "name containing dot must return None"
    );
}

// ── user_template_path: path traversal is rejected ───────────────────────

#[test]
fn user_template_path_with_dotdot_is_rejected() {
    let config_dir = PathBuf::from("/home/user/.config/cc-myasl");
    // Names containing ".." must be rejected — user_template_path returns None.
    assert!(
        user_template_path(&config_dir, "../../../etc/passwd").is_none(),
        "dotdot traversal must return None"
    );
    assert!(
        user_template_path(&config_dir, "..").is_none(),
        "bare dotdot must return None"
    );
    assert!(
        user_template_path(&config_dir, "foo/bar").is_none(),
        "slash in name must return None"
    );
    assert!(
        user_template_path(&config_dir, ".hidden").is_none(),
        "leading dot must return None"
    );
    assert!(
        user_template_path(&config_dir, r"foo\bar").is_none(),
        "backslash in name must return None"
    );
}

#[test]
fn user_template_path_valid_names_return_some() {
    let config_dir = PathBuf::from("/home/user/.config/cc-myasl");
    assert_eq!(
        user_template_path(&config_dir, "compact"),
        Some(PathBuf::from(
            "/home/user/.config/cc-myasl/templates/compact.json"
        ))
    );
    assert_eq!(
        user_template_path(&config_dir, "my-template"),
        Some(PathBuf::from(
            "/home/user/.config/cc-myasl/templates/my-template.json"
        ))
    );
    assert_eq!(
        user_template_path(&config_dir, "my_template_2"),
        Some(PathBuf::from(
            "/home/user/.config/cc-myasl/templates/my_template_2.json"
        ))
    );
}

#[test]
fn user_template_path_empty_name_is_rejected() {
    let config_dir = PathBuf::from("/home/user/.config/cc-myasl");
    assert!(
        user_template_path(&config_dir, "").is_none(),
        "empty name must return None"
    );
}

// ── resolve: empty template name falls through ────────────────────────────

#[test]
fn resolve_empty_template_name_falls_through_to_embedded() {
    let _guard = lock_config();
    let dir = tempdir().unwrap();
    let prior_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    unsafe { std::env::set_var("XDG_CONFIG_HOME", dir.path()) };
    let prior_sc = std::env::var("STATUSLINE_CONFIG").ok();
    unsafe { std::env::remove_var("STATUSLINE_CONFIG") };

    let mut args = empty_args();
    args.template_name = Some("".to_owned());
    let mut trace = Trace::default();
    let cfg = resolve(&args, &mut trace);

    match prior_xdg {
        Some(v) => unsafe { std::env::set_var("XDG_CONFIG_HOME", v) },
        None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
    }
    match prior_sc {
        Some(v) => unsafe { std::env::set_var("STATUSLINE_CONFIG", v) },
        None => unsafe { std::env::remove_var("STATUSLINE_CONFIG") },
    }

    // An empty template name matches no built-in and no user file.
    // It should fall all the way through to the embedded default.
    assert!(
        !cfg.lines.is_empty(),
        "empty template name must fall through and return a non-empty config"
    );
    // trace.error records "unknown template name: "
    assert!(
        trace.error.is_some(),
        "empty template name must record an error in trace"
    );
}

// ── resolve: STATUSLINE_CONFIG = " " (whitespace-only, not empty) ─────────

#[test]
fn resolve_whitespace_only_statusline_config_is_treated_as_path() {
    // The production check is `!env_val.is_empty()` — a single space passes
    // that check and is fed to from_file(Path::new(" ")). Since " " does not
    // exist as a file, from_file returns an error and resolve falls through.
    // This test pins that observable behaviour (whitespace is NOT silently
    // ignored — the error is recorded in trace).
    let _guard = lock_config();
    let dir = tempdir().unwrap();
    let prior_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    unsafe { std::env::set_var("XDG_CONFIG_HOME", dir.path()) };
    let prior_sc = std::env::var("STATUSLINE_CONFIG").ok();
    unsafe { std::env::set_var("STATUSLINE_CONFIG", " ") };

    let args = empty_args();
    let mut trace = Trace::default();
    let cfg = resolve(&args, &mut trace);

    match prior_xdg {
        Some(v) => unsafe { std::env::set_var("XDG_CONFIG_HOME", v) },
        None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
    }
    match prior_sc {
        Some(v) => unsafe { std::env::set_var("STATUSLINE_CONFIG", v) },
        None => unsafe { std::env::remove_var("STATUSLINE_CONFIG") },
    }

    // Whitespace-only value fails as a path → falls back, config is non-empty.
    assert!(
        !cfg.lines.is_empty(),
        "fallback must return non-empty config"
    );
    // The " " path attempt records an error.
    assert!(
        trace.error.is_some(),
        "whitespace STATUSLINE_CONFIG must record a parse error in trace"
    );
    // Must NOT be sourced from Env (the Env parse failed).
    assert_ne!(
        trace.config_source,
        Some(ConfigSource::Env),
        "whitespace STATUSLINE_CONFIG must not produce Env source"
    );
}

// ── resolve: env nonexistent path ─────────────────────────────────────────

#[test]
fn resolve_env_nonexistent_path_records_error_and_falls_back() {
    let _guard = lock_config();
    let dir = tempdir().unwrap();
    let prior_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    unsafe { std::env::set_var("XDG_CONFIG_HOME", dir.path()) };
    let prior_sc = std::env::var("STATUSLINE_CONFIG").ok();
    unsafe {
        std::env::set_var(
            "STATUSLINE_CONFIG",
            "/tmp/cc-myasl-definitely-does-not-exist-99887766.json",
        )
    };

    let args = empty_args();
    let mut trace = Trace::default();
    let cfg = resolve(&args, &mut trace);

    match prior_xdg {
        Some(v) => unsafe { std::env::set_var("XDG_CONFIG_HOME", v) },
        None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
    }
    match prior_sc {
        Some(v) => unsafe { std::env::set_var("STATUSLINE_CONFIG", v) },
        None => unsafe { std::env::remove_var("STATUSLINE_CONFIG") },
    }

    assert!(
        !cfg.lines.is_empty(),
        "fallback must return non-empty config"
    );
    assert!(
        trace.error.is_some(),
        "nonexistent env path must record error in trace"
    );
    assert_ne!(
        trace.config_source,
        Some(ConfigSource::Env),
        "nonexistent env path must not produce Env source"
    );
}

// ── resolve: XDG_CONFIG_HOME → nonexistent dir → graceful ────────────────

#[test]
fn resolve_xdg_nonexistent_dir_falls_back_to_embedded() {
    let _guard = lock_config();
    let prior_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    // Point at a directory that will never exist.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", "/tmp/cc-myasl-no-such-xdg-home-112233") };
    let prior_sc = std::env::var("STATUSLINE_CONFIG").ok();
    unsafe { std::env::remove_var("STATUSLINE_CONFIG") };

    let args = empty_args();
    let mut trace = Trace::default();
    let cfg = resolve(&args, &mut trace);

    match prior_xdg {
        Some(v) => unsafe { std::env::set_var("XDG_CONFIG_HOME", v) },
        None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
    }
    match prior_sc {
        Some(v) => unsafe { std::env::set_var("STATUSLINE_CONFIG", v) },
        None => unsafe { std::env::remove_var("STATUSLINE_CONFIG") },
    }

    assert!(!cfg.lines.is_empty(), "must fall back to embedded default");
    assert_eq!(
        trace.config_source,
        Some(ConfigSource::Embedded),
        "nonexistent XDG dir must produce Embedded source"
    );
}

// ── resolve: layer 1 error does not set config_source to CliPath ──────────

#[test]
fn resolve_layer1_failure_does_not_set_config_source() {
    // When args.config_path points at a nonexistent file, layer 1 fails.
    // config_source must NOT be set to "CliPath".
    let mut args = empty_args();
    args.config_path = Some(PathBuf::from(
        "/tmp/cc-myasl-layer1-fail-no-such-file-44556677.json",
    ));
    let mut trace = Trace::default();
    let _ = resolve(&args, &mut trace);

    assert_ne!(
        trace.config_source,
        Some(ConfigSource::CliPath),
        "failed layer 1 must not record CliPath as config_source"
    );
    assert!(
        trace.error.is_some(),
        "failed layer 1 must record error in trace"
    );
}

// ── print_config: canonical $schema overrides user-supplied value ─────────

#[test]
fn print_config_overrides_existing_schema_url_with_canonical() {
    let cfg = Config {
        schema_url: Some("https://example.com/my-custom-schema.json".to_owned()),
        ..Config::default()
    };
    let out = print_config(&cfg);
    assert!(
        out.contains("jLAM-ERR"),
        "print_config must inject canonical $schema regardless of existing value"
    );
    assert!(
        !out.contains("my-custom-schema"),
        "print_config must replace user-supplied $schema with canonical URL"
    );
}

// ── print_config: zero-line config serialises cleanly ─────────────────────

#[test]
fn print_config_empty_lines_produces_valid_json() {
    let cfg = Config {
        schema_url: None,
        powerline: false,
        lines: vec![],
    };
    let out = print_config(&cfg);
    // Must parse back as a valid Config.
    let back: Config = serde_json::from_str(&out)
        .expect("print_config on zero-line Config must produce parseable JSON");
    assert!(
        back.lines.is_empty(),
        "round-tripped zero-line config must have empty lines"
    );
    assert!(
        out.contains("$schema"),
        "even a zero-line config must include $schema"
    );
}

// ── print_config: round-trip stability ────────────────────────────────────

#[test]
fn print_config_round_trip_is_stable() {
    // print → parse → print again must produce byte-identical output.
    let cfg = Config::default();
    let first = print_config(&cfg);
    let back: Config = serde_json::from_str(&first).expect("first print must parse");
    let second = print_config(&back);
    assert_eq!(
        first, second,
        "print_config must be stable across a round-trip (first != second)"
    );
}

// ── invariant: tests_b does not import api or cache ───────────────────────

#[test]
fn tests_b_does_not_import_api_or_cache() {
    let src = std::fs::read_to_string("src/config/tests_b.rs").unwrap_or_default();
    let api_import = ["use crate", "::", "api"].concat();
    let cache_import = ["use crate", "::", "cache"].concat();
    assert!(!src.contains(&api_import), "tests_b must not import api");
    assert!(
        !src.contains(&cache_import),
        "tests_b must not import cache"
    );
}

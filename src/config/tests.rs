use super::*;
use crate::args::Args;
use crate::debug::Trace;
use std::path::PathBuf;
use tempfile::tempdir;

// ── helpers ───────────────────────────────────────────────────────────────

fn empty_args() -> Args {
    Args::default()
}

fn write_config(dir: &std::path::Path, filename: &str, content: &str) -> PathBuf {
    let path = dir.join(filename);
    std::fs::write(&path, content).expect("write test config");
    path
}

/// Acquire CONFIG_MUTEX, recovering from a poisoned lock so a panicking test
/// in a sibling thread does not permanently block all subsequent tests.
fn lock_config() -> std::sync::MutexGuard<'static, ()> {
    CONFIG_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

const MINIMAL_CONFIG_JSON: &str =
    r#"{"lines":[{"separator":"","segments":[{"template":"hello"}]}]}"#;

const CORRUPT_JSON: &str = r#"{ not valid json"#;

const TOO_MANY_LINES_JSON: &str = r#"{
  "lines": [
    {"segments":[{"template":"a"}]},
    {"segments":[{"template":"b"}]},
    {"segments":[{"template":"c"}]},
    {"segments":[{"template":"d"}]}
  ]
}"#;

// ── from_file ─────────────────────────────────────────────────────────────

#[test]
fn from_file_valid() {
    let dir = tempdir().unwrap();
    let path = write_config(dir.path(), "config.json", MINIMAL_CONFIG_JSON);
    let cfg = from_file(&path).expect("valid JSON must parse");
    assert_eq!(cfg.lines.len(), 1);
    assert_eq!(cfg.lines[0].segments.len(), 1);
}

#[test]
fn from_file_missing_returns_config_parse_error() {
    let path = PathBuf::from("/tmp/cc-myasl-test-nonexistent-12345/no.json");
    let err = from_file(&path).unwrap_err();
    assert!(
        matches!(err, crate::error::Error::ConfigParse(_)),
        "missing file must yield ConfigParse, got {err:?}"
    );
}

#[test]
fn from_file_corrupt_json_returns_config_parse_error() {
    let dir = tempdir().unwrap();
    let path = write_config(dir.path(), "bad.json", CORRUPT_JSON);
    let err = from_file(&path).unwrap_err();
    assert!(
        matches!(err, crate::error::Error::ConfigParse(_)),
        "corrupt JSON must yield ConfigParse, got {err:?}"
    );
}

#[test]
fn from_file_invalid_config_returns_config_parse_error() {
    let dir = tempdir().unwrap();
    let path = write_config(dir.path(), "big.json", TOO_MANY_LINES_JSON);
    let err = from_file(&path).unwrap_err();
    assert!(
        matches!(err, crate::error::Error::ConfigParse(_)),
        "invalid config (too many lines) must yield ConfigParse"
    );
}

// ── Config::default ───────────────────────────────────────────────────────

#[test]
fn config_default_is_valid() {
    let mut cfg = Config::default();
    let result = cfg.validate_and_clamp();
    assert!(
        result.is_ok(),
        "Config::default() must validate without errors: {result:?}"
    );
    assert!(
        !cfg.lines.is_empty(),
        "default config must have at least one line"
    );
    assert!(
        !cfg.lines[0].segments.is_empty(),
        "default config line 0 must have at least one segment"
    );
}

// ── user_template_path ────────────────────────────────────────────────────

#[test]
fn user_template_path_builds_correct_path() {
    let dir = PathBuf::from("/home/user/.config/cc-myasl");
    let p = user_template_path(&dir, "compact");
    assert_eq!(
        p,
        PathBuf::from("/home/user/.config/cc-myasl/templates/compact.json")
    );
}

// ── print_config ──────────────────────────────────────────────────────────

#[test]
fn print_config_includes_schema_field() {
    let cfg = Config::default();
    let out = print_config(&cfg);
    assert!(
        out.contains("$schema"),
        "print_config must include $schema field"
    );
    assert!(
        out.contains("jLAM-ERR"),
        "print_config $schema must reference jLAM-ERR repo"
    );
}

#[test]
fn print_config_is_deterministic() {
    let cfg = Config::default();
    let a = print_config(&cfg);
    let b = print_config(&cfg);
    assert_eq!(a, b, "print_config must be deterministic");
}

#[test]
fn print_config_round_trips() {
    let cfg = Config::default();
    let json = print_config(&cfg);
    let back: Config = serde_json::from_str(&json).expect("print_config output must be valid JSON");
    // Compare structurally (schema_url will differ — print_config injects it)
    assert_eq!(back.lines, cfg.lines, "round-tripped lines must match");
}

#[test]
fn print_config_is_pretty_json() {
    let cfg = Config::default();
    let out = print_config(&cfg);
    assert!(
        out.contains('\n'),
        "print_config must produce pretty (multi-line) JSON"
    );
}

// ── resolve — layer 1: args.config_path ───────────────────────────────────

#[test]
fn resolve_layer1_config_path() {
    let dir = tempdir().unwrap();
    let path = write_config(dir.path(), "my.json", MINIMAL_CONFIG_JSON);
    let mut args = empty_args();
    args.config_path = Some(path);
    let mut trace = Trace::default();
    let cfg = resolve(&args, &mut trace);
    assert_eq!(cfg.lines.len(), 1);
    assert_eq!(
        trace.config_source.as_deref(),
        Some("CliPath"),
        "trace must record CliPath"
    );
}

#[test]
fn resolve_layer1_corrupt_falls_back_to_embedded() {
    let dir = tempdir().unwrap();
    let path = write_config(dir.path(), "bad.json", CORRUPT_JSON);
    let mut args = empty_args();
    args.config_path = Some(path);
    let mut trace = Trace::default();
    let cfg = resolve(&args, &mut trace);
    assert!(!cfg.lines.is_empty(), "fallback config must have lines");
    assert!(
        trace.error.is_some(),
        "error must be recorded in trace on parse failure"
    );
}

// ── resolve — layer 2: args.template (built-in) ───────────────────────────

#[test]
fn resolve_layer2_template_builtin() {
    let mut args = empty_args();
    args.template = Some("compact".to_owned());
    let mut trace = Trace::default();
    let cfg = resolve(&args, &mut trace);
    assert!(
        !cfg.lines.is_empty(),
        "compact built-in must return a config with lines"
    );
    assert_eq!(
        trace.config_source.as_deref(),
        Some("CliTemplate"),
        "trace must record CliTemplate for built-in"
    );
}

#[test]
fn resolve_layer2_unknown_template_falls_through() {
    let _guard = lock_config();
    let prior = std::env::var("STATUSLINE_CONFIG").ok();
    std::env::remove_var("STATUSLINE_CONFIG");

    let mut args = empty_args();
    args.template = Some("nonexistent_template_xyz".to_owned());
    let mut trace = Trace::default();
    let cfg = resolve(&args, &mut trace);

    match prior {
        Some(v) => std::env::set_var("STATUSLINE_CONFIG", v),
        None => std::env::remove_var("STATUSLINE_CONFIG"),
    }

    assert!(!cfg.lines.is_empty());
}

// ── resolve — layer 2: user template shadows built-in ────────────────────

#[test]
fn resolve_layer2_user_template_shadows_builtin() {
    let _guard = lock_config();
    let dir = tempdir().unwrap();

    // Write a sentinel config to <tempdir>/cc-myasl/templates/default.json
    let cc_dir = dir.path().join("cc-myasl");
    let templates_dir = cc_dir.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    let sentinel_json =
        r#"{"lines":[{"separator":"","segments":[{"template":"SENTINEL_USER_TEMPLATE"}]}]}"#;
    std::fs::write(templates_dir.join("default.json"), sentinel_json).unwrap();

    let prior_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    std::env::set_var("XDG_CONFIG_HOME", dir.path());
    let prior_sc = std::env::var("STATUSLINE_CONFIG").ok();
    std::env::remove_var("STATUSLINE_CONFIG");

    let mut args = empty_args();
    args.template = Some("default".to_owned());
    let mut trace = Trace::default();
    let cfg = resolve(&args, &mut trace);

    match prior_xdg {
        Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
    match prior_sc {
        Some(v) => std::env::set_var("STATUSLINE_CONFIG", v),
        None => std::env::remove_var("STATUSLINE_CONFIG"),
    }

    assert_eq!(cfg.lines.len(), 1);
    let seg = &cfg.lines[0].segments[0];
    if let Segment::Template(t) = seg {
        assert_eq!(
            t.template, "SENTINEL_USER_TEMPLATE",
            "user template must shadow built-in"
        );
    } else {
        panic!("expected Template segment");
    }
    assert_eq!(
        trace.config_source.as_deref(),
        Some("CliTemplate"),
        "trace must record CliTemplate"
    );
}

// ── resolve — layer 3: STATUSLINE_CONFIG env var ──────────────────────────

#[test]
fn resolve_layer3_env_var() {
    let _guard = lock_config();
    let dir = tempdir().unwrap();
    let path = write_config(dir.path(), "env.json", MINIMAL_CONFIG_JSON);

    let prior = std::env::var("STATUSLINE_CONFIG").ok();
    std::env::set_var("STATUSLINE_CONFIG", path.to_str().unwrap());

    let args = empty_args();
    let mut trace = Trace::default();
    let cfg = resolve(&args, &mut trace);

    match prior {
        Some(v) => std::env::set_var("STATUSLINE_CONFIG", v),
        None => std::env::remove_var("STATUSLINE_CONFIG"),
    }

    assert_eq!(cfg.lines.len(), 1);
    assert_eq!(
        trace.config_source.as_deref(),
        Some("Env"),
        "trace must record Env"
    );
}

#[test]
fn resolve_layer3_empty_env_var_skipped() {
    let _guard = lock_config();
    let prior = std::env::var("STATUSLINE_CONFIG").ok();
    std::env::set_var("STATUSLINE_CONFIG", "");

    let args = empty_args();
    let mut trace = Trace::default();
    let _ = resolve(&args, &mut trace);

    match prior {
        Some(v) => std::env::set_var("STATUSLINE_CONFIG", v),
        None => std::env::remove_var("STATUSLINE_CONFIG"),
    }

    assert_ne!(
        trace.config_source.as_deref(),
        Some("Env"),
        "empty STATUSLINE_CONFIG must be skipped"
    );
}

#[test]
fn resolve_layer3_env_corrupt_falls_back() {
    let _guard = lock_config();
    let dir = tempdir().unwrap();
    let path = write_config(dir.path(), "corrupt.json", CORRUPT_JSON);

    let prior = std::env::var("STATUSLINE_CONFIG").ok();
    std::env::set_var("STATUSLINE_CONFIG", path.to_str().unwrap());

    let args = empty_args();
    let mut trace = Trace::default();
    let cfg = resolve(&args, &mut trace);

    match prior {
        Some(v) => std::env::set_var("STATUSLINE_CONFIG", v),
        None => std::env::remove_var("STATUSLINE_CONFIG"),
    }

    assert!(
        !cfg.lines.is_empty(),
        "fallback must return non-empty config"
    );
    assert!(
        trace.error.is_some(),
        "error must be recorded for corrupt env config"
    );
}

// ── resolve — layer 4: default config file ────────────────────────────────

#[test]
fn resolve_layer4_default_file() {
    let _guard = lock_config();
    let dir = tempdir().unwrap();

    let cc_dir = dir.path().join("cc-myasl");
    std::fs::create_dir_all(&cc_dir).unwrap();
    std::fs::write(cc_dir.join("config.json"), MINIMAL_CONFIG_JSON).unwrap();

    let prior_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    std::env::set_var("XDG_CONFIG_HOME", dir.path());
    let prior_sc = std::env::var("STATUSLINE_CONFIG").ok();
    std::env::remove_var("STATUSLINE_CONFIG");

    let args = empty_args();
    let mut trace = Trace::default();
    let cfg = resolve(&args, &mut trace);

    match prior_xdg {
        Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
    match prior_sc {
        Some(v) => std::env::set_var("STATUSLINE_CONFIG", v),
        None => std::env::remove_var("STATUSLINE_CONFIG"),
    }

    assert_eq!(cfg.lines.len(), 1);
    assert_eq!(
        trace.config_source.as_deref(),
        Some("DefaultFile"),
        "trace must record DefaultFile"
    );
}

// ── resolve — layer 5: embedded default ──────────────────────────────────

#[test]
fn resolve_layer5_embedded_default() {
    let _guard = lock_config();
    let dir = tempdir().unwrap();

    let prior_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    std::env::set_var("XDG_CONFIG_HOME", dir.path());
    let prior_sc = std::env::var("STATUSLINE_CONFIG").ok();
    std::env::remove_var("STATUSLINE_CONFIG");

    let args = empty_args();
    let mut trace = Trace::default();
    let cfg = resolve(&args, &mut trace);

    match prior_xdg {
        Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
    match prior_sc {
        Some(v) => std::env::set_var("STATUSLINE_CONFIG", v),
        None => std::env::remove_var("STATUSLINE_CONFIG"),
    }

    assert!(!cfg.lines.is_empty(), "embedded default must have lines");
    assert_eq!(
        trace.config_source.as_deref(),
        Some("Embedded"),
        "trace must record Embedded"
    );
}

// ── precedence: layer 1 wins over layer 2 ────────────────────────────────

#[test]
fn resolve_layer1_wins_over_layer2() {
    let dir = tempdir().unwrap();
    let path = write_config(dir.path(), "override.json", MINIMAL_CONFIG_JSON);

    let mut args = empty_args();
    args.config_path = Some(path);
    args.template = Some("compact".to_owned());
    let mut trace = Trace::default();
    let cfg = resolve(&args, &mut trace);

    assert_eq!(cfg.lines.len(), 1, "layer1 config has 1 line");
    assert_eq!(trace.config_source.as_deref(), Some("CliPath"));
}

// ── invariant: no api/cache imports in config/*.rs ───────────────────────

#[test]
fn config_module_does_not_depend_on_api_or_cache() {
    use std::fs;
    let files = [
        "src/config/mod.rs",
        "src/config/schema.rs",
        "src/config/builtins.rs",
        "src/config/render.rs",
        "src/config/tests.rs",
    ];
    let api_import = ["use crate", "::", "api"].concat();
    let cache_import = ["use crate", "::", "cache"].concat();
    for f in &files {
        let src = fs::read_to_string(f).unwrap_or_default();
        assert!(!src.contains(&api_import), "{f} has forbidden dep on api");
        assert!(
            !src.contains(&cache_import),
            "{f} has forbidden dep on cache"
        );
    }
}

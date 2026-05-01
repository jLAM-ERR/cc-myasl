/// Follow-up fix tests for config/mod.rs (Task 5 followup).
///
/// Covers: non-object JSON rejection (Fix 2), trace.error on corrupt user
/// template (Fix 3), trace.error on corrupt default file (Fix 4), ConfigSource
/// enum as_str values (Fix 5).
use super::*;
use crate::args::Args;
use crate::debug::{ConfigSource, Trace};
use tempfile::tempdir;

// ── helpers ───────────────────────────────────────────────────────────────

fn empty_args() -> Args {
    Args::default()
}

fn lock_config() -> std::sync::MutexGuard<'static, ()> {
    CONFIG_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

// ── ConfigSource: all variant strings match expected values ───────────────

#[test]
fn config_source_as_str_all_variants() {
    assert_eq!(ConfigSource::CliPath.as_str(), "CliPath");
    assert_eq!(ConfigSource::CliTemplate.as_str(), "CliTemplate");
    assert_eq!(ConfigSource::Env.as_str(), "Env");
    assert_eq!(ConfigSource::DefaultFile.as_str(), "DefaultFile");
    assert_eq!(ConfigSource::Embedded.as_str(), "Embedded");
}

// ── ConfigSource: strings match what resolve writes to trace ──────────────

#[test]
fn config_source_trace_strings_match_enum_as_str() {
    let expected = ["CliPath", "CliTemplate", "Env", "DefaultFile", "Embedded"];
    let actual = [
        ConfigSource::CliPath.as_str(),
        ConfigSource::CliTemplate.as_str(),
        ConfigSource::Env.as_str(),
        ConfigSource::DefaultFile.as_str(),
        ConfigSource::Embedded.as_str(),
    ];
    assert_eq!(expected, actual);
}

// ── from_file: non-object JSON types return ConfigParse error (Fix 2) ───────

#[test]
fn from_file_array_json_returns_config_parse_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("array.json");
    std::fs::write(&path, b"[]").unwrap();
    let err = from_file(&path).unwrap_err();
    assert!(
        matches!(err, crate::error::Error::ConfigParse(_)),
        "JSON array must yield ConfigParse, got {err:?}"
    );
    assert!(
        err.to_string().contains("array"),
        "error message must mention 'array': {err}"
    );
}

#[test]
fn from_file_string_json_returns_config_parse_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("str.json");
    std::fs::write(&path, b"\"hello\"").unwrap();
    let err = from_file(&path).unwrap_err();
    assert!(
        matches!(err, crate::error::Error::ConfigParse(_)),
        "JSON string must yield ConfigParse, got {err:?}"
    );
}

#[test]
fn from_file_number_json_returns_config_parse_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("num.json");
    std::fs::write(&path, b"42").unwrap();
    let err = from_file(&path).unwrap_err();
    assert!(
        matches!(err, crate::error::Error::ConfigParse(_)),
        "JSON number must yield ConfigParse, got {err:?}"
    );
}

// ── resolve_user_template: corrupt file records trace.error (Fix 3) ──────

#[test]
fn resolve_user_template_corrupt_file_records_error() {
    let _guard = lock_config();
    let dir = tempdir().unwrap();

    let cc_dir = dir.path().join("cc-myasl");
    let templates_dir = cc_dir.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    // Write a corrupt JSON file at the expected template path.
    std::fs::write(templates_dir.join("mytemplate.json"), b"{ not valid").unwrap();

    let prior_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    std::env::set_var("XDG_CONFIG_HOME", dir.path());
    let prior_sc = std::env::var("STATUSLINE_CONFIG").ok();
    std::env::remove_var("STATUSLINE_CONFIG");

    let mut args = empty_args();
    args.template_name = Some("mytemplate".to_owned());
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

    assert!(
        !cfg.lines.is_empty(),
        "must fall back to a non-empty config"
    );
    assert!(
        trace.error.is_some(),
        "corrupt user template must record error in trace"
    );
}

// ── load_default_config_file: corrupt file records trace.error (Fix 4) ───

#[test]
fn resolve_layer4_corrupt_default_file_records_error() {
    let _guard = lock_config();
    let dir = tempdir().unwrap();

    let cc_dir = dir.path().join("cc-myasl");
    std::fs::create_dir_all(&cc_dir).unwrap();
    // Write a corrupt JSON file at the default config location.
    std::fs::write(cc_dir.join("config.json"), b"{ not valid").unwrap();

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

    assert!(
        !cfg.lines.is_empty(),
        "must fall back to a non-empty config"
    );
    assert!(
        trace.error.is_some(),
        "corrupt default config file must record error in trace"
    );
    assert_ne!(
        trace.config_source,
        Some(ConfigSource::DefaultFile),
        "corrupt file must not be recorded as DefaultFile source"
    );
}

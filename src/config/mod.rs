pub mod builtins;
pub mod render;
pub mod schema;

pub use builtins::lookup;
pub use schema::{
    Config, FlexSegment, Line, Segment, TemplateSegment, ValidationError, ValidationWarning,
    MAX_LINES, MAX_PADDING,
};

use crate::args::Args;
use crate::debug::{ConfigSource, Trace};
use crate::error::Error;
use std::path::{Path, PathBuf};

const SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/cc-myasl.schema.json";

/// Returns `Some(path)` where path is `<config_dir>/templates/<name>.json`,
/// or `None` if `name` contains unsafe characters (`/`, `\`, `..`, or a
/// leading `.`).  Names must be simple identifiers: alphanumeric, `-`, `_`.
pub(crate) fn user_template_path(config_dir: &Path, name: &str) -> Option<PathBuf> {
    if !is_safe_template_name(name) {
        return None;
    }
    Some(config_dir.join("templates").join(format!("{name}.json")))
}

/// A safe template name contains only ASCII alphanumeric chars, `-`, or `_`,
/// and must be non-empty.  No slashes, dots, backslashes, or other separators.
fn is_safe_template_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Serialize `config` as pretty JSON with the canonical `$schema` field injected.
pub fn print_config(config: &Config) -> String {
    let mut c = config.clone();
    c.schema_url = Some(SCHEMA_URL.to_owned());
    serde_json::to_string_pretty(&c).unwrap_or_else(|_| "{}".to_owned())
}

/// Load `path`, parse JSON, validate, and return a `Config`.
///
/// Returns `Err` if the file cannot be read, is not a JSON object, or fails
/// validation.  Bubbles up errors so the caller can record a trace event and
/// fall back.
pub fn from_file(path: &Path) -> Result<Config, Error> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::ConfigParse(format!("cannot read {}: {e}", path.display())))?;
    // Reject non-object JSON before attempting struct deserialization to avoid
    // serde's #[serde(default)] silently filling an empty Config from an array
    // or scalar document.
    let raw: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| Error::ConfigParse(format!("JSON parse error in {}: {e}", path.display())))?;
    if !raw.is_object() {
        let kind = json_type_name(&raw);
        return Err(Error::ConfigParse(format!(
            "expected JSON object, got {kind} in {}",
            path.display()
        )));
    }
    let mut cfg: Config = serde_json::from_value(raw)
        .map_err(|e| Error::ConfigParse(format!("JSON parse error in {}: {e}", path.display())))?;
    cfg.validate_and_clamp().map_err(|errs| {
        Error::ConfigParse(format!("invalid config in {}: {errs:?}", path.display()))
    })?;
    Ok(cfg)
}

fn json_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Resolve the active config using the precedence ladder:
///
/// 1. `args.config_path` (Some) → load file
/// 2. `args.template_name` (Some) → check user templates dir, then built-ins
/// 3. `STATUSLINE_CONFIG` env var (non-empty) → load file
/// 4. Default file at `<config_dir>/cc-myasl/config.json`
/// 5. Embedded default (`builtins::lookup("default")`)
///
/// Never returns an error — every failure falls back to the next layer,
/// emitting a trace event when `trace` is Some.
pub fn resolve(args: &Args, trace: &mut Trace) -> Config {
    // Step 1: explicit --config path
    if let Some(ref path) = args.config_path {
        match from_file(path) {
            Ok(cfg) => {
                trace.config_source = Some(ConfigSource::CliPath);
                return cfg;
            }
            Err(e) => {
                trace.error = Some(e.to_string());
            }
        }
    }

    // Step 2: --template name
    if let Some(ref name) = args.template_name {
        // Check user templates dir first
        if let Some(user_cfg) = resolve_user_template(name, trace) {
            trace.config_source = Some(ConfigSource::CliTemplate);
            return user_cfg;
        }
        // Then built-ins
        if let Some(cfg) = builtins::lookup(name) {
            trace.config_source = Some(ConfigSource::CliTemplate);
            return cfg;
        }
        // Unknown name — record as non-fatal and fall through
        if trace.error.is_none() {
            trace.error = Some(format!("unknown template name: {name}"));
        }
    }

    // Step 3: STATUSLINE_CONFIG env var
    if let Ok(env_val) = std::env::var("STATUSLINE_CONFIG") {
        if !env_val.is_empty() {
            match from_file(Path::new(&env_val)) {
                Ok(cfg) => {
                    trace.config_source = Some(ConfigSource::Env);
                    return cfg;
                }
                Err(e) => {
                    trace.error = Some(e.to_string());
                }
            }
        }
    }

    // Step 4: default config file
    if let Some(cfg) = load_default_config_file(trace) {
        return cfg;
    }

    // Step 5: embedded default
    trace.config_source = Some(ConfigSource::Embedded);
    Config::default()
}

/// Load user template by name from the config dir.  Returns `None` on any
/// failure, recording parse errors in `trace.error`.
fn resolve_user_template(name: &str, trace: &mut Trace) -> Option<Config> {
    let config_dir = project_config_dir()?;
    let path = user_template_path(&config_dir, name)?;
    match from_file(&path) {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            // Only record parse errors (not "file not found") — a missing user
            // template is expected; a corrupt one deserves a trace entry.
            if path.exists() {
                trace.error = Some(e.to_string());
            }
            None
        }
    }
}

fn load_default_config_file(trace: &mut Trace) -> Option<Config> {
    let config_dir = project_config_dir()?;
    let path = config_dir.join("config.json");
    match from_file(&path) {
        Ok(cfg) => {
            trace.config_source = Some(ConfigSource::DefaultFile);
            Some(cfg)
        }
        Err(e) => {
            // File not found is silent (normal — user has no default file).
            // Any other error (corrupt, permission denied) is recorded.
            if path.exists() {
                trace.error = Some(e.to_string());
            }
            None
        }
    }
}

fn project_config_dir() -> Option<PathBuf> {
    // Honour XDG_CONFIG_HOME on all platforms for testability.
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join("cc-myasl"));
        }
    }
    directories::ProjectDirs::from("", "", "cc-myasl").map(|pd| pd.config_dir().to_owned())
}

/// Returns the user templates directory (`<config_dir>/templates`), or `None`
/// if the config dir cannot be determined.  The directory may not exist.
pub(crate) fn user_templates_dir() -> Option<PathBuf> {
    project_config_dir().map(|d| d.join("templates"))
}

impl Default for Config {
    fn default() -> Config {
        builtins::lookup("default").expect("built-in default must always exist")
    }
}

/// Shared mutex serializing tests that read or mutate `STATUSLINE_CONFIG`
/// or `XDG_CONFIG_HOME`.
#[cfg(test)]
pub(crate) static CONFIG_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
#[path = "tests.rs"]
mod config_tests;

#[cfg(test)]
#[path = "tests_b.rs"]
mod config_tests_b;

#[cfg(test)]
#[path = "tests_c.rs"]
mod config_tests_c;

#[cfg(test)]
#[path = "tests_d.rs"]
mod config_tests_d;

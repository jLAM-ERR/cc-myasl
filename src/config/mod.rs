pub mod builtins;
pub mod render;
pub mod schema;

pub use builtins::lookup;
pub use schema::{
    Config, FlexSegment, Line, Segment, TemplateSegment, ValidationError, ValidationWarning,
    MAX_LINES, MAX_PADDING,
};

use crate::args::Args;
use crate::debug::Trace;
use crate::error::Error;
use std::path::{Path, PathBuf};

const SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/cc-myasl.schema.json";

/// Which config-resolution layer produced the active config.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigSource {
    /// `--config <path>` flag.
    CliPath,
    /// `--template <name>` flag (user file or built-in).
    CliTemplate,
    /// `STATUSLINE_CONFIG` env var.
    Env,
    /// Default file at `<config_dir>/cc-myasl/config.json`.
    DefaultFile,
    /// Embedded built-in `default` template.
    Embedded,
}

impl ConfigSource {
    fn as_str(&self) -> &'static str {
        match self {
            ConfigSource::CliPath => "CliPath",
            ConfigSource::CliTemplate => "CliTemplate",
            ConfigSource::Env => "Env",
            ConfigSource::DefaultFile => "DefaultFile",
            ConfigSource::Embedded => "Embedded",
        }
    }
}

/// Returns the path `<config_dir>/templates/<name>.json`.
///
/// `config_dir` is the project-specific directory from `ProjectDirs`
/// (e.g. `~/.config/cc-myasl`), NOT its parent.
pub fn user_template_path(config_dir: &Path, name: &str) -> PathBuf {
    config_dir.join("templates").join(format!("{name}.json"))
}

/// Serialize `config` as pretty JSON with the canonical `$schema` field injected.
///
/// The `$schema` line is emitted first because `schema_url` is the first
/// field in `Config` (Rust struct field insertion order).
pub fn print_config(config: &Config) -> String {
    let mut c = config.clone();
    c.schema_url = Some(SCHEMA_URL.to_owned());
    serde_json::to_string_pretty(&c).unwrap_or_else(|_| "{}".to_owned())
}

/// Load `path`, parse JSON, validate, and return a `Config`.
///
/// Bubbles up errors so the caller (`resolve`) can record a trace event
/// and fall back to the default config.
pub fn from_file(path: &Path) -> Result<Config, Error> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::ConfigParse(format!("cannot read {}: {e}", path.display())))?;
    let mut cfg: Config = serde_json::from_str(&content)
        .map_err(|e| Error::ConfigParse(format!("JSON parse error in {}: {e}", path.display())))?;
    cfg.validate_and_clamp().map_err(|errs| {
        Error::ConfigParse(format!("invalid config in {}: {errs:?}", path.display()))
    })?;
    Ok(cfg)
}

/// Resolve the active config using the precedence ladder:
///
/// 1. `args.config_path` (Some) → load file
/// 2. `args.template_name` (Some) → check user templates dir, then built-ins
/// 3. `STATUSLINE_CONFIG` env var (non-empty) → load file
/// 4. Default file at `<config_dir>/cc-myasl/config.json`
/// 5. Embedded default (`builtins::lookup("default")`)
///
/// When step 1 and step 2 are both set (`--config X --template Y`),
/// `config_path` wins (step 1 comes first).  Task 7 populates both fields;
/// this resolver is the single source of truth for precedence.
///
/// Never returns an error — every failure falls back to the next layer,
/// emitting a trace event when `trace` is Some.
pub fn resolve(args: &Args, trace: &mut Trace) -> Config {
    // Step 1: explicit --config path
    if let Some(ref path) = args.config_path {
        match from_file(path) {
            Ok(cfg) => {
                trace.config_source = Some(ConfigSource::CliPath.as_str().to_owned());
                return cfg;
            }
            Err(e) => {
                trace.error = Some(e.to_string());
            }
        }
    }

    // Step 2: --template name (args.template; Task 7 may rename to template_name)
    if let Some(ref name) = args.template {
        // Check user templates dir first
        if let Some(user_cfg) = resolve_user_template(name) {
            trace.config_source = Some(ConfigSource::CliTemplate.as_str().to_owned());
            return user_cfg;
        }
        // Then built-ins
        if let Some(cfg) = builtins::lookup(name) {
            trace.config_source = Some(ConfigSource::CliTemplate.as_str().to_owned());
            return cfg;
        }
        // Unknown name — fall through to next layer (record as non-fatal)
        if trace.error.is_none() {
            trace.error = Some(format!("unknown template name: {name}"));
        }
    }

    // Step 3: STATUSLINE_CONFIG env var
    if let Ok(env_val) = std::env::var("STATUSLINE_CONFIG") {
        if !env_val.is_empty() {
            match from_file(Path::new(&env_val)) {
                Ok(cfg) => {
                    trace.config_source = Some(ConfigSource::Env.as_str().to_owned());
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
    trace.config_source = Some(ConfigSource::Embedded.as_str().to_owned());
    Config::default()
}

fn resolve_user_template(name: &str) -> Option<Config> {
    let config_dir = project_config_dir()?;
    let path = user_template_path(&config_dir, name);
    from_file(&path).ok()
}

fn load_default_config_file(trace: &mut Trace) -> Option<Config> {
    let config_dir = project_config_dir()?;
    let path = config_dir.join("config.json");
    match from_file(&path) {
        Ok(cfg) => {
            trace.config_source = Some(ConfigSource::DefaultFile.as_str().to_owned());
            Some(cfg)
        }
        Err(_) => None,
    }
}

fn project_config_dir() -> Option<PathBuf> {
    // Honour XDG_CONFIG_HOME on all platforms for testability.
    // On macOS, `directories` uses ~/Library/Application Support; this override
    // lets integration tests pin the config dir via env var.
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join("cc-myasl"));
        }
    }
    directories::ProjectDirs::from("", "", "cc-myasl").map(|pd| pd.config_dir().to_owned())
}

impl Default for Config {
    fn default() -> Config {
        builtins::lookup("default").expect("built-in default must always exist")
    }
}

/// Shared mutex serializing tests that read or mutate `STATUSLINE_CONFIG`
/// or `XDG_CONFIG_HOME`.  Mirrors `creds::HOME_MUTEX`, `format::ENV_MUTEX`,
/// and `config::render::COLS_MUTEX` — one mutex per logical env-var group.
#[cfg(test)]
pub(crate) static CONFIG_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
#[path = "tests.rs"]
mod config_tests;

#[cfg(test)]
#[path = "tests_b.rs"]
mod config_tests_b;

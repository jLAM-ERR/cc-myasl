//! Hand-rolled CLI argument parser.
//!
//! `parse` takes the argument list **after** `argv[0]` (the program name).
//! The caller is responsible for stripping `argv[0]` before passing the slice.
//!
//! # Example
//! ```
//! let raw: Vec<String> = std::env::args().skip(1).collect();
//! let args = cc_myasl::args::parse(&raw);
//! ```

#[derive(Debug, Default, PartialEq)]
pub struct Args {
    /// `--config <PATH>` or `--config=<PATH>` — explicit config file path.
    pub config_path: Option<std::path::PathBuf>,
    /// `--template <NAME>` or `--template=<NAME>`
    pub template_name: Option<String>,
    /// `--print-config` — print the resolved config as pretty JSON and exit.
    pub print_config: bool,
    /// `--debug`
    pub debug: bool,
    /// `--check`
    pub check: bool,
    /// `--version` or `-V`
    pub version: bool,
    /// `--help` or `-h`
    pub help: bool,
    /// Retained for Task 8 removal — no longer parsed from CLI.
    #[doc(hidden)]
    pub format: Option<String>,
    /// Any flag or value not recognized by the parser, preserved in order.
    /// A dangling `--config` or `--template` (no following value) is pushed
    /// here as the raw flag string so the caller can inspect it; the
    /// corresponding field is left `None`.
    pub unknown: Vec<String>,
}

/// Parse a pre-sliced argument list (everything after `argv[0]`).
///
/// One forward pass; never panics on any input.
///
/// Precedence (highest → lowest) for config resolution is handled by
/// `config::resolve`, not here:
///   1. `--config <path>`    (explicit file)
///   2. `--template <name>`  (built-in or user template)
///   3. `STATUSLINE_CONFIG`  (env var, same as --config)
///   4. Default config file  (`<config_dir>/cc-myasl/config.json`)
///   5. Embedded default     (`builtins::lookup("default")`)
///
/// Passing both `--config` and `--template` is allowed; both fields are
/// populated and the resolver picks `config_path` (step 1).
pub fn parse(args: &[String]) -> Args {
    let mut out = Args::default();
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        // Split `--key=value` form.
        if let Some(stripped) = arg.strip_prefix("--") {
            if let Some(eq_pos) = stripped.find('=') {
                let key = &stripped[..eq_pos];
                let val = &stripped[eq_pos + 1..];
                match key {
                    "config" => out.config_path = Some(std::path::PathBuf::from(val)),
                    "template" => out.template_name = Some(val.to_owned()),
                    "print-config" => {
                        out.print_config = true;
                        // `=VALUE` suffix on a boolean flag is unexpected; treat
                        // the whole token as unknown to avoid silent misuse.
                        if !val.is_empty() {
                            out.unknown.push(arg.clone());
                            out.print_config = false;
                        }
                    }
                    "debug" => {
                        out.debug = true;
                        if !val.is_empty() {
                            out.unknown.push(arg.clone());
                            out.debug = false;
                        }
                    }
                    "check" => {
                        out.check = true;
                        if !val.is_empty() {
                            out.unknown.push(arg.clone());
                            out.check = false;
                        }
                    }
                    "version" => {
                        out.version = true;
                        if !val.is_empty() {
                            out.unknown.push(arg.clone());
                            out.version = false;
                        }
                    }
                    "help" => {
                        out.help = true;
                        if !val.is_empty() {
                            out.unknown.push(arg.clone());
                            out.help = false;
                        }
                    }
                    _ => out.unknown.push(arg.clone()),
                }
                continue;
            }

            // Plain `--key` form.
            match stripped {
                "config" => {
                    // Peek: consume only if next token is not a flag.
                    if iter
                        .as_slice()
                        .first()
                        .is_some_and(|v| !v.starts_with("--"))
                    {
                        out.config_path = Some(std::path::PathBuf::from(iter.next().unwrap()));
                    } else {
                        out.unknown.push(arg.clone());
                    }
                }
                "template" => {
                    if iter
                        .as_slice()
                        .first()
                        .is_some_and(|v| !v.starts_with("--"))
                    {
                        out.template_name = Some(iter.next().unwrap().clone());
                    } else {
                        out.unknown.push(arg.clone());
                    }
                }
                "print-config" => out.print_config = true,
                "debug" => out.debug = true,
                "check" => out.check = true,
                "version" => out.version = true,
                "help" => out.help = true,
                _ => out.unknown.push(arg.clone()),
            }
        } else if arg == "-V" {
            out.version = true;
        } else if arg == "-h" {
            out.help = true;
        } else {
            out.unknown.push(arg.clone());
        }
    }

    out
}

#[cfg(test)]
#[path = "args_tests.rs"]
mod tests;

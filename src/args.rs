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
                "config" => match iter.next() {
                    Some(val) => out.config_path = Some(std::path::PathBuf::from(val)),
                    None => {
                        out.unknown.push(arg.clone());
                    }
                },
                "template" => match iter.next() {
                    Some(val) => out.template_name = Some(val.clone()),
                    None => {
                        out.unknown.push(arg.clone());
                    }
                },
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
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn empty_argv() {
        assert_eq!(parse(&[]), Args::default());
    }

    #[test]
    fn debug_alone() {
        let a = parse(&s(&["--debug"]));
        assert!(a.debug);
        assert!(!a.check);
        assert!(!a.version);
        assert!(!a.help);
        assert!(!a.print_config);
        assert!(a.config_path.is_none());
        assert!(a.template_name.is_none());
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn check_alone() {
        let a = parse(&s(&["--check"]));
        assert!(a.check);
        assert!(!a.debug);
    }

    // ── --config ─────────────────────────────────────────────────────────────

    #[test]
    fn config_space() {
        let a = parse(&s(&["--config", "/tmp/my.json"]));
        assert_eq!(
            a.config_path,
            Some(std::path::PathBuf::from("/tmp/my.json"))
        );
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn config_equals() {
        let a = parse(&s(&["--config=/home/user/cc.json"]));
        assert_eq!(
            a.config_path,
            Some(std::path::PathBuf::from("/home/user/cc.json"))
        );
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn config_dangling() {
        // `--config` with no following argument: config_path stays None,
        // the raw flag is pushed into unknown.
        let a = parse(&s(&["--config"]));
        assert!(a.config_path.is_none());
        assert_eq!(a.unknown, vec!["--config".to_string()]);
    }

    // ── --template ───────────────────────────────────────────────────────────

    #[test]
    fn template_space() {
        let a = parse(&s(&["--template", "default"]));
        assert_eq!(a.template_name, Some("default".into()));
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn template_equals() {
        let a = parse(&s(&["--template=compact"]));
        assert_eq!(a.template_name, Some("compact".into()));
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn template_dangling() {
        let a = parse(&s(&["--template"]));
        assert!(a.template_name.is_none());
        assert_eq!(a.unknown, vec!["--template".to_string()]);
    }

    // ── --print-config ───────────────────────────────────────────────────────

    #[test]
    fn print_config_alone() {
        let a = parse(&s(&["--print-config"]));
        assert!(a.print_config);
        assert!(!a.debug);
        assert!(a.config_path.is_none());
        assert!(a.template_name.is_none());
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn print_config_with_value_suffix_is_unknown() {
        // Boolean flags with `=VALUE` are treated as unknown.
        let a = parse(&s(&["--print-config=yes"]));
        assert!(!a.print_config);
        assert_eq!(a.unknown, vec!["--print-config=yes".to_string()]);
    }

    // ── combined flags ───────────────────────────────────────────────────────

    #[test]
    fn config_and_template_both_populate() {
        let a = parse(&s(&["--config", "/my.json", "--template", "minimal"]));
        assert_eq!(a.config_path, Some(std::path::PathBuf::from("/my.json")));
        assert_eq!(a.template_name, Some("minimal".into()));
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn print_config_combined_with_config() {
        let a = parse(&s(&["--print-config", "--config", "/x.json"]));
        assert!(a.print_config);
        assert_eq!(a.config_path, Some(std::path::PathBuf::from("/x.json")));
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn debug_and_print_config_and_template() {
        let a = parse(&s(&["--debug", "--print-config", "--template", "bars"]));
        assert!(a.debug);
        assert!(a.print_config);
        assert_eq!(a.template_name, Some("bars".into()));
        assert!(a.unknown.is_empty());
    }

    // ── --format no longer accepted ──────────────────────────────────────────

    #[test]
    fn format_flag_goes_to_unknown() {
        // --format is no longer a recognized flag; goes to unknown.
        let a = parse(&s(&["--format", "foo"]));
        assert!(a.format.is_none());
        // Both the flag and its value land in unknown.
        assert!(a.unknown.contains(&"--format".to_string()));
    }

    #[test]
    fn format_equals_goes_to_unknown() {
        let a = parse(&s(&["--format=foo"]));
        assert!(a.format.is_none());
        assert!(a.unknown.contains(&"--format=foo".to_string()));
    }

    // ── other flags ──────────────────────────────────────────────────────────

    #[test]
    fn version_long() {
        let a = parse(&s(&["--version"]));
        assert!(a.version);
    }

    #[test]
    fn version_short() {
        let a = parse(&s(&["-V"]));
        assert!(a.version);
    }

    #[test]
    fn help_long() {
        let a = parse(&s(&["--help"]));
        assert!(a.help);
    }

    #[test]
    fn help_short() {
        let a = parse(&s(&["-h"]));
        assert!(a.help);
    }

    #[test]
    fn unknown_flag() {
        let a = parse(&s(&["--bogus"]));
        assert_eq!(a.unknown, vec!["--bogus".to_string()]);
        assert!(!a.debug);
        assert!(!a.check);
        assert!(!a.version);
        assert!(!a.help);
    }

    #[test]
    fn multiple_unknowns_ordered() {
        let a = parse(&s(&["--foo", "--bar", "--baz"]));
        assert_eq!(
            a.unknown,
            vec![
                "--foo".to_string(),
                "--bar".to_string(),
                "--baz".to_string()
            ]
        );
    }

    #[test]
    fn positional_arg_goes_to_unknown() {
        let a = parse(&s(&["somefile"]));
        assert_eq!(a.unknown, vec!["somefile".to_string()]);
    }

    // ── adversarial boundary cases ───────────────────────────────────────────

    #[test]
    fn config_equals_empty_value_sets_empty_path() {
        // `--config=` → PathBuf::from(""). Pin current behavior.
        let a = parse(&s(&["--config="]));
        assert_eq!(a.config_path, Some(std::path::PathBuf::from("")));
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn config_double_equals_treated_as_path_with_leading_equals() {
        // `--config==/foo`: key="config", val="=/foo".
        let a = parse(&s(&["--config==/foo"]));
        assert_eq!(a.config_path, Some(std::path::PathBuf::from("=/foo")));
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn config_repeated_last_wins() {
        let a = parse(&s(&["--config", "/first.json", "--config", "/second.json"]));
        assert_eq!(
            a.config_path,
            Some(std::path::PathBuf::from("/second.json"))
        );
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn config_repeated_last_wins_interleaved_with_template() {
        let a = parse(&s(&[
            "--config",
            "/a.json",
            "--template",
            "default",
            "--config",
            "/b.json",
        ]));
        assert_eq!(a.config_path, Some(std::path::PathBuf::from("/b.json")));
        assert_eq!(a.template_name, Some("default".into()));
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn print_config_equals_empty_is_accepted() {
        // Empty val on boolean flag → accepted (same as bare flag).
        let a = parse(&s(&["--print-config="]));
        assert!(a.print_config);
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn print_config_repeated_is_idempotent() {
        let a = parse(&s(&["--print-config", "--print-config"]));
        assert!(a.print_config);
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn short_c_not_recognized() {
        let a = parse(&s(&["-c", "/tmp/x.json"]));
        assert!(a.config_path.is_none());
        assert!(a.unknown.contains(&"-c".to_string()));
    }

    #[test]
    fn uppercase_config_flag_not_recognized() {
        let a = parse(&s(&["--CONFIG", "/tmp/x.json"]));
        assert!(a.config_path.is_none());
        assert!(a.unknown.contains(&"--CONFIG".to_string()));
    }

    #[test]
    fn config_single_dash_treated_as_literal_path() {
        // `-` has no special stdin meaning; parsed as a literal path.
        let a = parse(&s(&["--config", "-"]));
        assert_eq!(a.config_path, Some(std::path::PathBuf::from("-")));
        assert!(a.unknown.is_empty());
    }

    #[test]
    #[ignore = "bug: --config consumes the next flag token as its value when no path follows"]
    fn config_followed_immediately_by_another_flag_should_error() {
        // `--config --template foo` → parser sets config_path=Some("--template"),
        // template_name=None. Expected: config_path=None, template_name=Some("foo").
        let a = parse(&s(&["--config", "--template", "foo"]));
        assert!(a.config_path.is_none());
        assert_eq!(a.template_name, Some("foo".into()));
    }

    #[test]
    fn double_dash_not_an_end_of_options_sentinel() {
        // `--` is not treated as an end-of-options delimiter: it goes to unknown,
        // but subsequent flags continue to be parsed normally.
        let a = parse(&s(&["--", "--debug"]));
        assert!(a.debug);
        assert!(a.unknown.contains(&"--".to_string()));
        // `--debug` is still parsed, so it should NOT appear in unknown.
        assert!(!a.unknown.contains(&"--debug".to_string()));
    }

    #[test]
    fn unicode_path_preserved() {
        let path = "/tmp/\u{043a}\u{043e}\u{043d}\u{0444}\u{0438}\u{0433}.json";
        let a = parse(&s(&["--config", path]));
        assert_eq!(a.config_path, Some(std::path::PathBuf::from(path)));
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn template_equals_empty_value_sets_empty_string() {
        let a = parse(&s(&["--template="]));
        assert_eq!(a.template_name, Some(String::new()));
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn check_equals_nonempty_is_unknown() {
        let a = parse(&s(&["--check=yes"]));
        assert!(!a.check);
        assert_eq!(a.unknown, vec!["--check=yes".to_string()]);
    }

    #[test]
    fn version_equals_nonempty_is_unknown() {
        let a = parse(&s(&["--version=1"]));
        assert!(!a.version);
        assert_eq!(a.unknown, vec!["--version=1".to_string()]);
    }

    #[test]
    fn all_known_flags_together() {
        let a = parse(&s(&[
            "--debug",
            "--check",
            "--version",
            "--help",
            "--print-config",
            "--config",
            "/x.json",
            "--template",
            "compact",
        ]));
        assert!(a.debug && a.check && a.version && a.help && a.print_config);
        assert_eq!(a.config_path, Some(std::path::PathBuf::from("/x.json")));
        assert_eq!(a.template_name, Some("compact".into()));
        assert!(a.unknown.is_empty());
    }
}

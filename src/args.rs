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
    /// `--format <STR>` or `--format=<STR>`
    pub format: Option<String>,
    /// `--template <NAME>` or `--template=<NAME>`
    pub template: Option<String>,
    /// `--debug`
    pub debug: bool,
    /// `--check`
    pub check: bool,
    /// `--version` or `-V`
    pub version: bool,
    /// `--help` or `-h`
    pub help: bool,
    /// Any flag or value not recognized by the parser, preserved in order.
    /// A dangling `--format` (no following value) is pushed here as the raw
    /// flag string so the caller can inspect it; `format` is left `None`.
    pub unknown: Vec<String>,
}

/// Parse a pre-sliced argument list (everything after `argv[0]`).
///
/// One forward pass; never panics on any input.
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
                    "format" => out.format = Some(val.to_owned()),
                    "template" => out.template = Some(val.to_owned()),
                    "debug" => {
                        out.debug = true;
                        // `=VALUE` suffix on a boolean flag is unexpected; treat
                        // the whole token as unknown to avoid silent misuse.
                        if !val.is_empty() {
                            out.unknown.push(arg.clone());
                            out.debug = false; // revert — it was malformed
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
                "format" => {
                    match iter.next() {
                        Some(val) => out.format = Some(val.clone()),
                        None => {
                            // Dangling flag — push into unknown, leave format None.
                            out.unknown.push(arg.clone());
                        }
                    }
                }
                "template" => match iter.next() {
                    Some(val) => out.template = Some(val.clone()),
                    None => {
                        out.unknown.push(arg.clone());
                    }
                },
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
        assert!(a.format.is_none());
        assert!(a.template.is_none());
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn check_alone() {
        let a = parse(&s(&["--check"]));
        assert!(a.check);
        assert!(!a.debug);
    }

    #[test]
    fn format_space() {
        let a = parse(&s(&["--format", "foo"]));
        assert_eq!(a.format, Some("foo".into()));
    }

    #[test]
    fn format_equals() {
        let a = parse(&s(&["--format=foo"]));
        assert_eq!(a.format, Some("foo".into()));
    }

    #[test]
    fn template_space() {
        let a = parse(&s(&["--template", "default"]));
        assert_eq!(a.template, Some("default".into()));
    }

    #[test]
    fn template_equals() {
        let a = parse(&s(&["--template=compact"]));
        assert_eq!(a.template, Some("compact".into()));
    }

    #[test]
    fn combined_flags() {
        let a = parse(&s(&["--debug", "--check", "--format", "default"]));
        assert!(a.debug);
        assert!(a.check);
        assert_eq!(a.format, Some("default".into()));
        assert!(a.unknown.is_empty());
    }

    #[test]
    fn dangling_format() {
        // `--format` with no following argument: format stays None,
        // the raw flag is pushed into unknown.
        let a = parse(&s(&["--format"]));
        assert!(a.format.is_none());
        assert_eq!(a.unknown, vec!["--format".to_string()]);
    }

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

    #[test]
    fn format_equals_empty_value() {
        // `--format=` sets format to empty string (valid, caller decides).
        let a = parse(&s(&["--format="]));
        assert_eq!(a.format, Some("".into()));
    }
}

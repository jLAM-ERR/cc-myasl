//! Template tokenizer for the status-line format language.
//!
//! Converts a template string into a flat sequence of [`Token`]s.
//! Recognised syntax:
//! - `{name}` — a placeholder token; `name` must be non-empty and
//!   consist of ASCII alphanumerics or `_`.
//! - `{? … }` — an optional block; the inner string is tokenized
//!   recursively and the whole block collapses to the empty string
//!   if any contained placeholder is absent (handled by the renderer).
//! - Anything else is accumulated as [`Token::Text`].
//!
//! Unterminated `{…` or `{? …` sequences are treated as literal text.

/// A single unit produced by [`tokenize`].
#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    /// A literal text run.
    Text(String),
    /// A `{name}` placeholder.
    Placeholder(String),
    /// A `{? … }` optional block whose inner tokens are preserved.
    Optional(Vec<Token>),
}

/// Tokenize `template` into a [`Vec<Token>`].
///
/// Single-pass, char-by-char.  Never panics; unterminated sequences
/// degrade to [`Token::Text`].
pub fn tokenize(template: &str) -> Vec<Token> {
    tokenize_inner(template, false).0
}

/// Internal recursive tokenizer.
///
/// `inside_optional` is `true` when we are processing the inner body
/// of a `{? … }` block.  In that mode a bare `}` ends the block and
/// the function returns, leaving the caller to consume the `}`.
///
/// Returns `(tokens, chars_consumed)`.
fn tokenize_inner(input: &str, inside_optional: bool) -> (Vec<Token>, usize) {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut pos = 0usize;
    let mut tokens: Vec<Token> = Vec::new();
    let mut text_buf = String::new();

    macro_rules! flush_text {
        () => {
            if !text_buf.is_empty() {
                tokens.push(Token::Text(std::mem::take(&mut text_buf)));
            }
        };
    }

    while pos < len {
        if inside_optional && chars[pos] == '}' {
            // End of the optional block — do NOT consume the `}` here;
            // let the parent consume it.
            break;
        }

        if chars[pos] != '{' {
            text_buf.push(chars[pos]);
            pos += 1;
            continue;
        }

        // We have `{`.  Peek ahead.
        let brace_start = pos;

        // Check for `{?` (optional block marker).
        if pos + 1 < len && chars[pos + 1] == '?' {
            // Expect the pattern `{? <inner> }`.
            // The inner content starts right after `{?`.
            let inner_start = pos + 2; // skip `{?`

            // Collect the inner string as a sub-slice and recurse.
            let remaining: String = chars[inner_start..].iter().collect();
            let (inner_tokens, consumed) = tokenize_inner(&remaining, true);

            let inner_end = inner_start + consumed;

            // The recursive call stops *at* the closing `}` (if it exists).
            if inner_end < len && chars[inner_end] == '}' {
                // Well-formed optional block.
                flush_text!();
                tokens.push(Token::Optional(inner_tokens));
                pos = inner_end + 1; // skip closing `}`
            } else {
                // Unterminated `{? …` — emit as literal text.
                let raw: String = chars[brace_start..].iter().collect();
                text_buf.push_str(&raw);
                pos = len; // consume the rest as text
            }
            continue;
        }

        // Try to parse `{name}` placeholder.
        // Scan forward for a `}`, collecting only valid name chars.
        let name_start = pos + 1;
        let mut name_end = name_start;
        let mut valid_name = true;

        while name_end < len && chars[name_end] != '}' {
            let c = chars[name_end];
            if !c.is_ascii_alphanumeric() && c != '_' {
                valid_name = false;
                break;
            }
            name_end += 1;
        }

        if valid_name && name_end < len && chars[name_end] == '}' && name_end > name_start {
            // Good placeholder.
            flush_text!();
            let name: String = chars[name_start..name_end].iter().collect();
            tokens.push(Token::Placeholder(name));
            pos = name_end + 1; // skip closing `}`
        } else {
            // Unterminated or invalid `{` — emit `{` as literal text
            // and advance past it so we don't loop forever.
            text_buf.push('{');
            pos += 1;
        }
    }

    flush_text!();

    let chars_consumed = pos;
    (tokens, chars_consumed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_template_gives_empty_vec() {
        assert_eq!(tokenize(""), vec![]);
    }

    #[test]
    fn plain_text_gives_single_text_token() {
        let toks = tokenize("hello world");
        assert_eq!(toks, vec![Token::Text("hello world".into())]);
    }

    #[test]
    fn single_placeholder() {
        let toks = tokenize("a {five} b");
        assert_eq!(
            toks,
            vec![
                Token::Text("a ".into()),
                Token::Placeholder("five".into()),
                Token::Text(" b".into()),
            ]
        );
    }

    #[test]
    fn multiple_consecutive_placeholders() {
        let toks = tokenize("{a}{b}{c}");
        assert_eq!(
            toks,
            vec![
                Token::Placeholder("a".into()),
                Token::Placeholder("b".into()),
                Token::Placeholder("c".into()),
            ]
        );
    }

    #[test]
    fn optional_segment_containing_placeholder() {
        let toks = tokenize("x {? foo {bar} baz } y");
        assert_eq!(
            toks,
            vec![
                Token::Text("x ".into()),
                Token::Optional(vec![
                    Token::Text(" foo ".into()),
                    Token::Placeholder("bar".into()),
                    Token::Text(" baz ".into()),
                ]),
                Token::Text(" y".into()),
            ]
        );
    }

    #[test]
    fn nested_optional() {
        // {? outer {? inner {x} } }
        let toks = tokenize("{? outer {? inner {x} } }");
        assert_eq!(
            toks,
            vec![Token::Optional(vec![
                Token::Text(" outer ".into()),
                Token::Optional(vec![
                    Token::Text(" inner ".into()),
                    Token::Placeholder("x".into()),
                    Token::Text(" ".into()),
                ]),
                Token::Text(" ".into()),
            ])]
        );
    }

    #[test]
    fn unterminated_brace_becomes_literal() {
        // `{abc` with no closing `}` should be emitted as literal text.
        let toks = tokenize("hello {abc");
        assert_eq!(toks, vec![Token::Text("hello {abc".into())]);
    }

    #[test]
    fn unterminated_optional_becomes_literal() {
        let toks = tokenize("x {? foo bar");
        // The whole `{? foo bar` (no closing `}`) should be literal.
        assert_eq!(toks, vec![Token::Text("x {? foo bar".into())]);
    }

    #[test]
    fn placeholder_with_underscore() {
        let toks = tokenize("{five_hour_pct}");
        assert_eq!(toks, vec![Token::Placeholder("five_hour_pct".into())]);
    }

    #[test]
    fn invalid_placeholder_chars_become_literal() {
        // `{foo bar}` has a space — not a valid placeholder.
        let toks = tokenize("{foo bar}");
        // Falls back to literal `{` then the rest.
        // The `{` is emitted, then `foo bar}` as text.
        let combined: String = toks
            .iter()
            .map(|t| match t {
                Token::Text(s) => s.clone(),
                _ => String::new(),
            })
            .collect();
        assert_eq!(combined, "{foo bar}");
    }

    #[test]
    fn mixed_content() {
        let toks = tokenize("Model: {model} · {?  ({cwd}) }");
        // Just assert counts and rough structure.
        assert!(toks.len() >= 3);
        assert!(matches!(toks[0], Token::Text(_)));
        assert!(matches!(toks[1], Token::Placeholder(ref n) if n == "model"));
    }
}

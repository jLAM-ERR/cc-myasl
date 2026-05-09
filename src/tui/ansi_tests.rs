use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

use super::ansi_to_lines;

// ── helpers ──────────────────────────────────────────────────────────────────

fn bold() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}
fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}
fn reversed() -> Style {
    Style::default().add_modifier(Modifier::REVERSED)
}
fn fg(c: Color) -> Style {
    Style::default().fg(c)
}
fn bg(c: Color) -> Style {
    Style::default().bg(c)
}

fn span_text(lines: &[ratatui::text::Line<'static>], idx: usize) -> String {
    lines[idx]
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect()
}

fn find_span<'a>(lines: &'a [ratatui::text::Line<'static>], content: &str) -> &'a Span<'static> {
    lines[0]
        .spans
        .iter()
        .find(|s| s.content == content)
        .unwrap_or_else(|| panic!("no span with content {:?}", content))
}

// ── plain text ────────────────────────────────────────────────────────────────

#[test]
fn plain_text_no_escape() {
    let lines = ansi_to_lines("hello");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].spans.len(), 1);
    assert_eq!(lines[0].spans[0].content, "hello");
    assert_eq!(lines[0].spans[0].style, Style::default());
}

#[test]
fn empty_input() {
    let lines = ansi_to_lines("");
    assert_eq!(lines.len(), 1);
    let total: usize = lines[0].spans.iter().map(|s| s.content.len()).sum();
    assert_eq!(total, 0);
}

// ── SGR color codes ───────────────────────────────────────────────────────────

#[test]
fn fg_red() {
    let lines = ansi_to_lines("\x1b[31mhello\x1b[0m");
    assert_eq!(find_span(&lines, "hello").style, fg(Color::Red));
}

#[test]
fn bg_blue() {
    let lines = ansi_to_lines("\x1b[44mworld\x1b[0m");
    assert_eq!(find_span(&lines, "world").style, bg(Color::Blue));
}

#[test]
fn fg_bright_green() {
    let lines = ansi_to_lines("\x1b[92mhi\x1b[0m");
    assert_eq!(find_span(&lines, "hi").style, fg(Color::LightGreen));
}

#[test]
fn bg_bright_red() {
    let lines = ansi_to_lines("\x1b[101mtest\x1b[0m");
    assert_eq!(find_span(&lines, "test").style, bg(Color::LightRed));
}

#[test]
fn all_standard_fg_colors() {
    let expected = [
        Color::Black,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::White,
    ];
    for (i, &color) in expected.iter().enumerate() {
        let code = 30 + i as u8;
        let input = format!("\x1b[{code}mx\x1b[0m");
        let lines = ansi_to_lines(&input);
        assert_eq!(
            find_span(&lines, "x").style,
            fg(color),
            "code {code} → {color:?}"
        );
    }
}

#[test]
fn all_standard_bg_colors() {
    let expected = [
        Color::Black,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::White,
    ];
    for (i, &color) in expected.iter().enumerate() {
        let code = 40 + i as u8;
        let input = format!("\x1b[{code}mx\x1b[0m");
        let lines = ansi_to_lines(&input);
        assert_eq!(
            find_span(&lines, "x").style,
            bg(color),
            "code {code} → {color:?}"
        );
    }
}

// ── SGR modifier codes ────────────────────────────────────────────────────────

#[test]
fn bold_modifier() {
    assert_eq!(
        find_span(&ansi_to_lines("\x1b[1mBOLD\x1b[0m"), "BOLD").style,
        bold()
    );
}

#[test]
fn dim_modifier() {
    assert_eq!(
        find_span(&ansi_to_lines("\x1b[2mDIM\x1b[0m"), "DIM").style,
        dim()
    );
}

#[test]
fn reverse_modifier() {
    assert_eq!(
        find_span(&ansi_to_lines("\x1b[7mREV\x1b[0m"), "REV").style,
        reversed()
    );
}

#[test]
fn reset_clears_style() {
    let lines = ansi_to_lines("\x1b[31mred\x1b[0mplain");
    assert_eq!(find_span(&lines, "red").style, fg(Color::Red));
    assert_eq!(find_span(&lines, "plain").style, Style::default());
}

#[test]
fn code22_clears_bold_and_dim() {
    let lines = ansi_to_lines("\x1b[1mB\x1b[22mN\x1b[0m");
    assert!(find_span(&lines, "B").style.add_modifier == Modifier::BOLD);
    let n = find_span(&lines, "N").style;
    assert!(!n.add_modifier.contains(Modifier::BOLD));
    assert!(!n.add_modifier.contains(Modifier::DIM));
}

#[test]
fn code27_clears_reverse() {
    let lines = ansi_to_lines("\x1b[7mR\x1b[27mN\x1b[0m");
    assert!(
        find_span(&lines, "R")
            .style
            .add_modifier
            .contains(Modifier::REVERSED)
    );
    assert!(
        !find_span(&lines, "N")
            .style
            .add_modifier
            .contains(Modifier::REVERSED)
    );
}

#[test]
fn code39_clears_fg() {
    let lines = ansi_to_lines("\x1b[31m\x1b[39mplain");
    assert_eq!(find_span(&lines, "plain").style.fg, Some(Color::Reset));
}

#[test]
fn code49_clears_bg() {
    let lines = ansi_to_lines("\x1b[41m\x1b[49mplain");
    assert_eq!(find_span(&lines, "plain").style.bg, Some(Color::Reset));
}

// ── multi-line ────────────────────────────────────────────────────────────────

#[test]
fn multi_line_split_on_newline() {
    let lines = ansi_to_lines("line1\nline2");
    assert_eq!(lines.len(), 2);
    assert_eq!(span_text(&lines, 0), "line1");
    assert_eq!(span_text(&lines, 1), "line2");
}

#[test]
fn style_preserved_across_newline() {
    let lines = ansi_to_lines("\x1b[31mred\nstill-red\x1b[0m");
    assert_eq!(lines.len(), 2);
    assert_eq!(find_span(&lines[1..], "still-red").style, fg(Color::Red));
}

// ── malformed / edge cases ────────────────────────────────────────────────────

#[test]
fn malformed_escape_no_bracket() {
    let lines = ansi_to_lines("\x1bXhello");
    let text = span_text(&lines, 0);
    assert_eq!(text, "Xhello");
    assert!(!text.contains('\x1b'));
}

#[test]
fn malformed_escape_bare_esc() {
    assert_eq!(span_text(&ansi_to_lines("hello\x1b"), 0), "hello");
}

#[test]
fn unterminated_csi() {
    let lines = ansi_to_lines("before\x1b[31");
    assert_eq!(lines.len(), 1);
    assert_eq!(span_text(&lines, 0), "before");
}

#[test]
fn empty_csi_m_is_reset() {
    let lines = ansi_to_lines("\x1b[31mred\x1b[mplain");
    assert_eq!(find_span(&lines, "plain").style, Style::default());
}

#[test]
fn semicolon_only_csi_is_no_op() {
    // `\x1b[;m` → empty parts, all skipped; style unchanged (not a reset).
    let lines = ansi_to_lines("\x1b[31mred\x1b[;mstill-red");
    assert_eq!(find_span(&lines, "still-red").style, fg(Color::Red));
}

#[test]
fn unknown_non_m_csi_skipped() {
    assert_eq!(span_text(&ansi_to_lines("a\x1b[2Jb"), 0), "ab");
}

#[test]
fn combined_fg_and_modifier() {
    let lines = ansi_to_lines("\x1b[1;32mhi\x1b[0m");
    let s = find_span(&lines, "hi").style;
    assert!(s.add_modifier.contains(Modifier::BOLD));
    assert_eq!(s.fg, Some(Color::Green));
}

// ── post-parse mutation smoke test ────────────────────────────────────────────

#[test]
fn post_parse_style_mutation() {
    let mut lines = ansi_to_lines("\x1b[31mhello\x1b[0m");
    for line in &mut lines {
        for span in &mut line.spans {
            span.style = span.style.add_modifier(Modifier::DIM);
        }
    }
    let s = find_span(&lines, "hello").style;
    assert_eq!(s.fg, Some(Color::Red));
    assert!(s.add_modifier.contains(Modifier::DIM));
}

#[test]
fn post_parse_mutation_smoke_frame() {
    use ratatui::{Terminal, backend::TestBackend, widgets::Paragraph};

    let mut lines = ansi_to_lines("\x1b[31mhello\x1b[0m");
    for line in &mut lines {
        for span in &mut line.spans {
            span.style = span.style.add_modifier(Modifier::DIM);
        }
    }
    let backend = TestBackend::new(20, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let para = Paragraph::new(lines.clone());
            f.render_widget(para, f.area());
        })
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    let rendered: String = (0..5)
        .map(|x| buf.cell((x, 0)).unwrap().symbol().to_string())
        .collect();
    assert_eq!(rendered, "hello");
}

// ── ownership invariant ───────────────────────────────────────────────────────

#[test]
fn spans_are_static_after_input_dropped() {
    let lines: Vec<ratatui::text::Line<'static>> = {
        let input = String::from("\x1b[32mgreen\x1b[0m");
        ansi_to_lines(&input)
    };
    assert_eq!(find_span(&lines, "green").style, fg(Color::Green));
}

// Compile-time check: return type must be Vec<Line<'static>>.
const _: fn() = || {
    let _: Vec<ratatui::text::Line<'static>> = ansi_to_lines("");
};

// ── single element / boundary ─────────────────────────────────────────────────

#[test]
fn single_newline() {
    assert_eq!(ansi_to_lines("\n").len(), 2);
}

#[test]
fn max_codes_all_bright_bg() {
    // 107 = bright white bg
    assert_eq!(
        find_span(&ansi_to_lines("\x1b[107mx\x1b[0m"), "x").style.bg,
        Some(Color::White)
    );
}

// ── UTF-8 preservation ────────────────────────────────────────────────────────

#[test]
fn utf8_powerline_chevron_preserved() {
    let input = "\x1b[31m\u{e0b0}main\x1b[0m";
    let lines = ansi_to_lines(input);
    let text: String = lines[0]
        .spans
        .iter()
        .filter(|s| s.style == fg(Color::Red))
        .map(|s| s.content.as_ref())
        .collect();
    assert_eq!(text, "\u{e0b0}main");
}

#[test]
fn utf8_multibyte_branch_name() {
    let lines = ansi_to_lines("\x1b[32mrésumé\x1b[0m");
    let span = lines[0]
        .spans
        .iter()
        .find(|s| s.content.contains('é'))
        .unwrap();
    assert_eq!(span.content.as_ref(), "résumé");
}

// ── parse-error skip (no silent reset) ───────────────────────────────────────

#[test]
fn overflow_code_does_not_reset_style() {
    // Overflowing code → skip, style unchanged (stays Red).
    let lines = ansi_to_lines("\x1b[31m\x1b[99999999999m hello");
    let span = find_span(&lines, " hello");
    assert_eq!(span.style, fg(Color::Red));
}

#[test]
fn garbage_code_does_not_reset_style() {
    // \x1b[abc m — 'a' is alphabetic so it terminates CSI as a non-m command → skipped.
    let lines = ansi_to_lines("\x1b[31m\x1b[abc m hello");
    for span in &lines[0].spans {
        if !span.content.is_empty() {
            assert_eq!(span.style.fg, Some(Color::Red));
        }
    }
}

// ── empty CSI reset ───────────────────────────────────────────────────────────

#[test]
fn empty_csi_resets_after_color() {
    let lines = ansi_to_lines("\x1b[31mred\x1b[mreset");
    assert_eq!(find_span(&lines, "reset").style, Style::default());
    assert_eq!(find_span(&lines, "red").style, fg(Color::Red));
}

// ── 256-color and RGB ─────────────────────────────────────────────────────────

#[test]
fn fg_256_color_indexed() {
    let lines = ansi_to_lines("\x1b[38;5;200mhello\x1b[0m");
    assert_eq!(
        find_span(&lines, "hello").style.fg,
        Some(Color::Indexed(200))
    );
}

#[test]
fn bg_256_color_indexed() {
    let lines = ansi_to_lines("\x1b[48;5;100mhello\x1b[0m");
    assert_eq!(
        find_span(&lines, "hello").style.bg,
        Some(Color::Indexed(100))
    );
}

#[test]
fn fg_rgb_color() {
    let lines = ansi_to_lines("\x1b[38;2;255;100;0mhello\x1b[0m");
    assert_eq!(
        find_span(&lines, "hello").style.fg,
        Some(Color::Rgb(255, 100, 0))
    );
}

#[test]
fn bg_rgb_color() {
    let lines = ansi_to_lines("\x1b[48;2;0;128;255mhello\x1b[0m");
    assert_eq!(
        find_span(&lines, "hello").style.bg,
        Some(Color::Rgb(0, 128, 255))
    );
}

#[test]
fn truncated_256_and_rgb_dropped_silently() {
    // Missing N in 38;5;N — no panic, text preserved.
    assert!(span_text(&ansi_to_lines("\x1b[38;5mhello"), 0).contains("hello"));
    // Missing B in 38;2;R;G;B — no panic, text preserved.
    assert!(span_text(&ansi_to_lines("\x1b[38;2;255;100mhello"), 0).contains("hello"));
}

#[test]
fn combined_256_color_and_modifier() {
    let lines = ansi_to_lines("\x1b[1;38;5;82mhi\x1b[0m");
    let s = find_span(&lines, "hi").style;
    assert!(s.add_modifier.contains(Modifier::BOLD));
    assert_eq!(s.fg, Some(Color::Indexed(82)));
}

// ── Span<'static> type check ──────────────────────────────────────────────────

#[test]
fn span_is_owned_string() {
    let lines = ansi_to_lines("abc");
    let span: &Span<'static> = &lines[0].spans[0];
    assert_eq!(span.content, "abc");
}

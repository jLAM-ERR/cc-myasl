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

// ── plain text ────────────────────────────────────────────────────────────────

#[test]
fn plain_text_no_escape() {
    let lines = ansi_to_lines("hello");
    assert_eq!(lines.len(), 1);
    let spans = &lines[0].spans;
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content, "hello");
    assert_eq!(spans[0].style, Style::default());
}

#[test]
fn empty_input() {
    let lines = ansi_to_lines("");
    // Always at least one line returned (even if empty).
    assert_eq!(lines.len(), 1);
    // The single line may have zero spans or one empty-content span — both OK.
    let total_chars: usize = lines[0].spans.iter().map(|s| s.content.len()).sum();
    assert_eq!(total_chars, 0);
}

// ── SGR color codes ───────────────────────────────────────────────────────────

#[test]
fn fg_red() {
    let lines = ansi_to_lines("\x1b[31mhello\x1b[0m");
    assert_eq!(lines.len(), 1);
    let spans = &lines[0].spans;
    // spans[0] = "hello" with fg Red
    let red_span = spans
        .iter()
        .find(|s| s.content == "hello")
        .expect("no hello span");
    assert_eq!(red_span.style, fg(Color::Red));
}

#[test]
fn bg_blue() {
    let lines = ansi_to_lines("\x1b[44mworld\x1b[0m");
    let spans = &lines[0].spans;
    let span = spans
        .iter()
        .find(|s| s.content == "world")
        .expect("no world span");
    assert_eq!(span.style, bg(Color::Blue));
}

#[test]
fn fg_bright_green() {
    // 92 = bright green fg
    let lines = ansi_to_lines("\x1b[92mhi\x1b[0m");
    let span = lines[0].spans.iter().find(|s| s.content == "hi").unwrap();
    assert_eq!(span.style, fg(Color::LightGreen));
}

#[test]
fn bg_bright_red() {
    // 101 = bright red bg
    let lines = ansi_to_lines("\x1b[101mtest\x1b[0m");
    let span = lines[0].spans.iter().find(|s| s.content == "test").unwrap();
    assert_eq!(span.style, bg(Color::LightRed));
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
        let span = lines[0].spans.iter().find(|s| s.content == "x").unwrap();
        assert_eq!(span.style, fg(color), "code {code} should map to {color:?}");
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
        let span = lines[0].spans.iter().find(|s| s.content == "x").unwrap();
        assert_eq!(span.style, bg(color), "code {code} should map to {color:?}");
    }
}

// ── SGR modifier codes ────────────────────────────────────────────────────────

#[test]
fn bold_modifier() {
    let lines = ansi_to_lines("\x1b[1mBOLD\x1b[0m");
    let span = lines[0].spans.iter().find(|s| s.content == "BOLD").unwrap();
    assert_eq!(span.style, bold());
}

#[test]
fn dim_modifier() {
    let lines = ansi_to_lines("\x1b[2mDIM\x1b[0m");
    let span = lines[0].spans.iter().find(|s| s.content == "DIM").unwrap();
    assert_eq!(span.style, dim());
}

#[test]
fn reverse_modifier() {
    let lines = ansi_to_lines("\x1b[7mREV\x1b[0m");
    let span = lines[0].spans.iter().find(|s| s.content == "REV").unwrap();
    assert_eq!(span.style, reversed());
}

#[test]
fn reset_clears_style() {
    let lines = ansi_to_lines("\x1b[31mred\x1b[0mplain");
    let spans = &lines[0].spans;
    let red = spans.iter().find(|s| s.content == "red").unwrap();
    let plain = spans.iter().find(|s| s.content == "plain").unwrap();
    assert_eq!(red.style, fg(Color::Red));
    assert_eq!(plain.style, Style::default());
}

#[test]
fn code22_clears_bold_and_dim() {
    // Set bold, then clear with 22.
    let lines = ansi_to_lines("\x1b[1mB\x1b[22mN\x1b[0m");
    let bold_span = lines[0].spans.iter().find(|s| s.content == "B").unwrap();
    let normal_span = lines[0].spans.iter().find(|s| s.content == "N").unwrap();
    assert!(bold_span.style.add_modifier == Modifier::BOLD);
    assert!(!normal_span.style.add_modifier.contains(Modifier::BOLD));
    assert!(!normal_span.style.add_modifier.contains(Modifier::DIM));
}

#[test]
fn code27_clears_reverse() {
    let lines = ansi_to_lines("\x1b[7mR\x1b[27mN\x1b[0m");
    let rev = lines[0].spans.iter().find(|s| s.content == "R").unwrap();
    let normal = lines[0].spans.iter().find(|s| s.content == "N").unwrap();
    assert!(rev.style.add_modifier.contains(Modifier::REVERSED));
    assert!(!normal.style.add_modifier.contains(Modifier::REVERSED));
}

#[test]
fn code39_clears_fg() {
    let lines = ansi_to_lines("\x1b[31m\x1b[39mplain");
    let span = lines[0]
        .spans
        .iter()
        .find(|s| s.content == "plain")
        .unwrap();
    // After 39, fg should be Reset (= default).
    assert_eq!(span.style.fg, Some(Color::Reset));
}

#[test]
fn code49_clears_bg() {
    let lines = ansi_to_lines("\x1b[41m\x1b[49mplain");
    let span = lines[0]
        .spans
        .iter()
        .find(|s| s.content == "plain")
        .unwrap();
    assert_eq!(span.style.bg, Some(Color::Reset));
}

// ── multi-line ────────────────────────────────────────────────────────────────

#[test]
fn multi_line_split_on_newline() {
    let lines = ansi_to_lines("line1\nline2");
    assert_eq!(lines.len(), 2);
    let t0: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
    let t1: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(t0, "line1");
    assert_eq!(t1, "line2");
}

#[test]
fn style_preserved_across_newline() {
    // Red set on line 1, text continues on line 2 — style should carry over.
    let lines = ansi_to_lines("\x1b[31mred\nstill-red\x1b[0m");
    assert_eq!(lines.len(), 2);
    let line2_span = lines[1]
        .spans
        .iter()
        .find(|s| s.content == "still-red")
        .unwrap();
    assert_eq!(line2_span.style, fg(Color::Red));
}

// ── malformed / edge cases ────────────────────────────────────────────────────

#[test]
fn malformed_escape_no_bracket() {
    // ESC not followed by '[' → ESC byte is skipped, text is preserved.
    let lines = ansi_to_lines("\x1bXhello");
    let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(text, "Xhello");
}

#[test]
fn malformed_escape_bare_esc() {
    // ESC at very end of string → just skipped.
    let lines = ansi_to_lines("hello\x1b");
    let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(text, "hello");
}

#[test]
fn unterminated_csi() {
    // CSI started but no terminating alphabetic byte → dropped, rest of input lost.
    let lines = ansi_to_lines("before\x1b[31");
    let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(text, "before");
}

#[test]
fn empty_csi_m_is_reset() {
    // `\x1b[m` with no params means reset (equivalent to `\x1b[0m`).
    let lines = ansi_to_lines("\x1b[31mred\x1b[mplain");
    let plain = lines[0]
        .spans
        .iter()
        .find(|s| s.content == "plain")
        .unwrap();
    assert_eq!(plain.style, Style::default());
}

#[test]
fn unknown_non_m_csi_skipped() {
    // `\x1b[2J` is a clear-screen command; should be silently skipped, text preserved.
    let lines = ansi_to_lines("a\x1b[2Jb");
    let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(text, "ab");
}

#[test]
fn combined_fg_and_modifier() {
    // `\x1b[1;32m` = bold + green fg in one CSI sequence.
    let lines = ansi_to_lines("\x1b[1;32mhi\x1b[0m");
    let span = lines[0].spans.iter().find(|s| s.content == "hi").unwrap();
    assert!(span.style.add_modifier.contains(Modifier::BOLD));
    assert_eq!(span.style.fg, Some(Color::Green));
}

// ── post-parse mutation smoke test ────────────────────────────────────────────

#[test]
fn post_parse_style_mutation() {
    let mut lines = ansi_to_lines("\x1b[31mhello\x1b[0m");
    // Overlay DIM on all spans after parsing.
    for line in &mut lines {
        for span in &mut line.spans {
            span.style = span.style.add_modifier(Modifier::DIM);
        }
    }
    let hello = lines[0]
        .spans
        .iter()
        .find(|s| s.content == "hello")
        .unwrap();
    // The original Red fg must still be there and DIM must be added.
    assert_eq!(hello.style.fg, Some(Color::Red));
    assert!(hello.style.add_modifier.contains(Modifier::DIM));
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

    // Verify the mutated spans can be rendered without panicking.
    let backend = TestBackend::new(20, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let para = Paragraph::new(lines.clone());
            f.render_widget(para, f.area());
        })
        .unwrap();

    // If we get here, rendering succeeded.
    let buf = terminal.backend().buffer().clone();
    // First 5 cells should contain "hello".
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
        // `input` is dropped here
    };
    let span = lines[0]
        .spans
        .iter()
        .find(|s| s.content == "green")
        .unwrap();
    assert_eq!(span.style, fg(Color::Green));
}

// ── single element / boundary ─────────────────────────────────────────────────

#[test]
fn single_newline() {
    let lines = ansi_to_lines("\n");
    assert_eq!(lines.len(), 2);
}

#[test]
fn max_codes_all_bright_bg() {
    // 107 = bright white bg (index 7 in BRIGHT)
    let lines = ansi_to_lines("\x1b[107mx\x1b[0m");
    let span = lines[0].spans.iter().find(|s| s.content == "x").unwrap();
    // bright white bg = Color::White
    assert_eq!(span.style.bg, Some(Color::White));
}

// Span::styled with a String must yield Span<'static>.
#[allow(dead_code)]
fn _assert_static(_: Span<'static>) {}

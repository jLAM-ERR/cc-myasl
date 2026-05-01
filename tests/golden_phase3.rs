//! Phase-3 golden tests: save flow, color rendering, Powerline rendering.

use cc_myasl::config;
use cc_myasl::config::render::render;
use cc_myasl::config::schema::{Config, Line, Segment, TemplateSegment};
use cc_myasl::format::placeholders::RenderCtx;
use cc_myasl::tui::save::{SaveError, save};
use tempfile::tempdir;

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_colored_segment(template: &str, fg: Option<&str>, bg: Option<&str>) -> Segment {
    let mut t = TemplateSegment::new(template);
    t.color = fg.map(str::to_owned);
    t.bg = bg.map(str::to_owned);
    Segment::Template(t)
}

fn one_line_config(segments: Vec<Segment>, powerline: bool) -> Config {
    Config {
        schema_url: None,
        powerline,
        lines: vec![Line {
            separator: " · ".to_owned(),
            segments,
        }],
    }
}

// ── golden_save_writes_valid_json ─────────────────────────────────────────────

#[test]
fn golden_save_writes_valid_json() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let cfg = one_line_config(
        vec![make_colored_segment("{model}", Some("red"), Some("blue"))],
        true,
    );

    save(&cfg, &path).expect("save must succeed");

    assert!(path.exists(), "output file must exist after save");

    let loaded = config::from_file(&path).expect("round-trip load must succeed");
    assert!(loaded.powerline, "powerline must round-trip");
    assert_eq!(loaded.lines.len(), 1, "one line must round-trip");
    if let Segment::Template(t) = &loaded.lines[0].segments[0] {
        assert_eq!(t.color.as_deref(), Some("red"), "color must round-trip");
        assert_eq!(t.bg.as_deref(), Some("blue"), "bg must round-trip");
    } else {
        panic!("expected Template segment");
    }
}

// ── golden_save_creates_backup ────────────────────────────────────────────────

#[test]
fn golden_save_creates_backup() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");
    let bak_path = dir.path().join("config.json.bak");

    let old_cfg = one_line_config(
        vec![Segment::Template(TemplateSegment::new("{five_left}%"))],
        false,
    );
    let old_json = config::print_config(&old_cfg);
    std::fs::write(&path, old_json.as_bytes()).unwrap();

    let new_cfg = one_line_config(
        vec![make_colored_segment("{model}", Some("cyan"), None)],
        false,
    );
    save(&new_cfg, &path).expect("save must succeed");

    assert!(bak_path.exists(), ".bak file must exist");
    let bak_content = std::fs::read_to_string(&bak_path).unwrap();
    assert!(
        bak_content.contains("five_left"),
        ".bak must contain old content; got: {bak_content:?}"
    );

    let new_content = std::fs::read_to_string(&path).unwrap();
    assert!(
        new_content.contains("model"),
        "target must contain new content; got: {new_content:?}"
    );
}

// ── golden_save_validates_before_write ───────────────────────────────────────

#[test]
fn golden_save_validates_before_write() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");

    let base_line = Line {
        separator: String::new(),
        segments: vec![Segment::Template(TemplateSegment::new("{model}"))],
    };
    // Bypass the validator by constructing directly with 4 lines (MAX_LINES = 3).
    let invalid_cfg = Config {
        schema_url: None,
        powerline: false,
        lines: vec![
            base_line.clone(),
            base_line.clone(),
            base_line.clone(),
            base_line.clone(),
        ],
    };
    assert_eq!(invalid_cfg.lines.len(), 4);

    let result = save(&invalid_cfg, &path);
    assert!(
        matches!(result, Err(SaveError::Validation(_))),
        "expected Validation error, got {:?}",
        result
    );
    assert!(
        !path.exists(),
        "target file must NOT be written on validation error"
    );
}

// ── golden_render_with_color_in_phase3_segments ───────────────────────────────

#[test]
fn golden_render_with_color_in_phase3_segments() {
    let cfg = one_line_config(
        vec![make_colored_segment("hello", Some("red"), Some("blue"))],
        false,
    );
    let ctx = RenderCtx::default();
    let out = render(&cfg, &ctx);

    assert!(
        out.contains("\x1b[31m"),
        "fg red escape must appear; got: {out:?}"
    );
    assert!(
        out.contains("\x1b[44m"),
        "bg blue escape must appear; got: {out:?}"
    );
    assert!(
        out.contains("\x1b[0m"),
        "reset escape must appear; got: {out:?}"
    );
    assert!(
        out.contains("hello"),
        "segment text must appear; got: {out:?}"
    );
}

// ── golden_render_powerline_mode ──────────────────────────────────────────────

#[test]
fn golden_render_powerline_mode() {
    use cc_myasl::config::render_powerline::CHEVRON;

    let cfg = one_line_config(
        vec![
            make_colored_segment("seg0", None, Some("red")),
            make_colored_segment("seg1", None, Some("blue")),
        ],
        true,
    );
    let ctx = RenderCtx::default();
    let out = render(&cfg, &ctx);

    let chevron_count = out.matches(CHEVRON).count();
    assert!(
        chevron_count >= 2,
        "must have at least 2 chevrons (1 between + 1 trailing); got {chevron_count}; out={out:?}"
    );

    // bg red = \x1b[41m, bg blue = \x1b[44m — both color transitions must appear
    assert!(out.contains("\x1b[41m"), "bg red must appear; got: {out:?}");
    assert!(
        out.contains("\x1b[44m"),
        "bg blue must appear; got: {out:?}"
    );
}

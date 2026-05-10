//! Middle-pane renderer — tab strip + preset checkbox list.
//!
//! Layout:
//!   Row 0 (inside border): tab strip — 8 categories separated by ` │ `.
//!                           Active tab is reverse-video.
//!   Rows 1+: checkbox list — `[x] LABEL    PREVIEW` or `[ ] LABEL    PREVIEW`.
//!
//! When `app.active_tab == Category::Appearance`, dispatches to
//! `panes::appearance::render` for the list area.
//!
//! Filter: when `app.mode == Mode::Filter`, only rows whose label OR template
//! contain `app.picker_filter` (case-insensitive substring) are shown.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::config::schema::{Config, Line as ConfigLine, Segment, TemplateSegment};
use crate::format::RenderCtx;
use crate::payload::Payload;
use crate::payload_mapping;
use crate::tui::ansi::ansi_to_lines;
use crate::tui::app4::{App, Focus};
use crate::tui::builder::BuilderSegment;
use crate::tui::catalog::{Category, by_category};

use super::appearance;

// ── fixture context ───────────────────────────────────────────────────────────

const FIXTURE_JSON: &str = include_str!("../preview_fixture.json");
const FIXTURE_NOW: u64 = 1_700_000_000;

fn fixture_ctx() -> RenderCtx {
    let payload: Payload = serde_json::from_str(FIXTURE_JSON)
        .expect("preview_fixture.json must be valid JSON — repo invariant");
    let mut ctx = payload_mapping::build_render_ctx(&payload, FIXTURE_NOW);
    if let Some(rl) = &payload.rate_limits {
        if let Some(fh) = &rl.five_hour {
            ctx.five_used = fh.used_percentage;
            ctx.five_reset_unix = fh.resets_at;
        }
        if let Some(sd) = &rl.seven_day {
            ctx.seven_used = sd.used_percentage;
            ctx.seven_reset_unix = sd.resets_at;
        }
    }
    ctx
}

// ── category display name ─────────────────────────────────────────────────────

fn category_label(c: Category) -> &'static str {
    match c {
        Category::Workspace => "workspace",
        Category::Git => "git",
        Category::SessionModel => "session/model",
        Category::Context => "context",
        Category::Tokens => "tokens",
        Category::Cost => "cost",
        Category::Rates => "rates",
        Category::Appearance => "Appearance",
    }
}

// ── live preview rendering for a preset template ──────────────────────────────

fn preset_preview_spans(template: &str, ctx: &RenderCtx) -> Vec<Span<'static>> {
    let cfg = Config {
        schema_url: None,
        lines: vec![ConfigLine {
            separator: String::new(),
            segments: vec![Segment::Template(TemplateSegment {
                template: template.to_owned(),
                padding: 0,
                hide_when_absent: false,
                color: None,
                bg: None,
            })],
        }],
        powerline: false,
        default_fg: None,
        default_bg: None,
    };
    let ansi_str = crate::config::render::render(&cfg, ctx);
    ansi_to_lines(&ansi_str)
        .into_iter()
        .next()
        .map(|l| l.spans)
        .unwrap_or_default()
}

// ── is preset active on current line ─────────────────────────────────────────

fn preset_is_active(app: &App, preset_id: &str) -> bool {
    app.builder
        .lines
        .get(app.active_line)
        .map(|l| {
            l.segments
                .iter()
                .any(|s| matches!(s, BuilderSegment::Preset { id, .. } if *id == preset_id))
        })
        .unwrap_or(false)
}

// ── filter visible presets ────────────────────────────────────────────────────

fn preset_matches_filter(label: &str, template: &str, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    let f = filter.to_lowercase();
    label.to_lowercase().contains(&f) || template.to_lowercase().contains(&f)
}

// ── public render entry point ─────────────────────────────────────────────────

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let (border_color, title_text) = if app.focus == Focus::Middle {
        (Color::Cyan, "▶ Configure ")
    } else {
        (Color::DarkGray, "  Configure ")
    };
    let title_style = if app.focus == Focus::Middle {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let block = Block::bordered()
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title_text, title_style));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    // Split inner area: 1 row for tab strip, rest for content.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    let tab_area = chunks[0];
    let list_area = chunks[1];

    // ── tab strip ─────────────────────────────────────────────────────────────
    let cats = Category::ordered();
    let mut tab_spans: Vec<Span<'static>> = Vec::new();
    for (i, &cat) in cats.iter().enumerate() {
        if i > 0 {
            tab_spans.push(Span::raw(" │ "));
        }
        let label = category_label(cat);
        let style = if cat == app.active_tab {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        tab_spans.push(Span::styled(label.to_owned(), style));
    }
    frame.render_widget(Paragraph::new(Line::from(tab_spans)), tab_area);

    // ── list area ─────────────────────────────────────────────────────────────
    if app.active_tab == Category::Appearance {
        appearance::render(frame, list_area, app);
        return;
    }

    let ctx = fixture_ctx();
    // Apply picker_filter in both Filter mode and Browsing (committed filter).
    let filter = app.picker_filter.as_str();

    let presets: Vec<_> = by_category(app.active_tab)
        .filter(|p| preset_matches_filter(p.label, p.template, filter))
        .collect();

    let mut rows: Vec<Line<'static>> = Vec::new();
    for (visible_idx, preset) in presets.iter().enumerate() {
        let checkbox = if preset_is_active(app, preset.id) {
            "[x] "
        } else {
            "[ ] "
        };

        // Label column — fixed 26 chars, padded with spaces.
        let label_col = format!("{:<26}", preset.label);

        // Preview column.
        let preview_spans = preset_preview_spans(preset.template, &ctx);
        let is_empty_preview =
            preview_spans.is_empty() || preview_spans.iter().all(|s| s.content.trim().is_empty());

        let row_style = if app.focus == Focus::Middle && app.picker_selected == visible_idx {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };

        let mut spans: Vec<Span<'static>> = vec![
            Span::styled(checkbox.to_owned(), row_style),
            Span::styled(label_col, row_style),
        ];

        if is_empty_preview {
            spans.push(Span::styled(
                "—".to_owned(),
                row_style.add_modifier(Modifier::DIM),
            ));
        } else if app.focus == Focus::Middle && app.picker_selected == visible_idx {
            // On the selected row we apply REVERSED to preview spans too.
            for mut s in preview_spans {
                s.style = s.style.add_modifier(Modifier::REVERSED);
                spans.push(s);
            }
        } else {
            spans.extend(preview_spans);
        }

        rows.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(rows), list_area);
}

#[cfg(test)]
#[path = "middle_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "middle_tests_b.rs"]
mod tests_b;

//! Top-pane renderer — live ANSI preview of the active BuilderState.
//!
//! Renders each real line segment-by-segment, then an optional virtual
//! `+ new line` row.  Applies Modifier::DIM to Custom segments and
//! Modifier::REVERSED to the cursor segment.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::config::schema::{Config, Line as ConfigLine, Segment, TemplateSegment};
use crate::format::RenderCtx;
use crate::tui::ansi::ansi_to_lines;
use crate::tui::app4::{App, Cursor, Focus};
use crate::tui::builder::BuilderSegment;
use crate::tui::catalog::lookup_by_id;

// ── fixture context ───────────────────────────────────────────────────────────

const FIXTURE_JSON: &str = include_str!("../preview_fixture.json");
const FIXTURE_NOW: u64 = 1_700_000_000;

fn fixture_ctx() -> RenderCtx {
    let payload: crate::payload::Payload = serde_json::from_str(FIXTURE_JSON).unwrap_or_default();
    let mut ctx = crate::payload_mapping::build_render_ctx(&payload, FIXTURE_NOW);
    // Map rate_limits → five_used / seven_used (mirrors main.rs hot path).
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

// ── per-segment config builder ────────────────────────────────────────────────

fn segment_to_config(seg: &BuilderSegment) -> Config {
    let template_seg = match seg {
        BuilderSegment::Preset { id, color, bg } => {
            let (template, hide_when_absent) = lookup_by_id(id)
                .map(|p| (p.template.to_owned(), p.hide_when_absent))
                .unwrap_or_else(|| (format!("{{{{{id}}}}}"), false));
            TemplateSegment {
                template,
                padding: 0,
                hide_when_absent,
                color: color.map(|c| c.as_str().to_owned()),
                bg: bg.map(|c| c.as_str().to_owned()),
            }
        }
        BuilderSegment::Custom {
            template,
            color,
            bg,
            padding,
            hide_when_absent,
        } => TemplateSegment {
            template: template.clone(),
            padding: *padding,
            hide_when_absent: *hide_when_absent,
            color: color.map(|c| c.as_str().to_owned()),
            bg: bg.map(|c| c.as_str().to_owned()),
        },
        BuilderSegment::FlexSpacer => TemplateSegment {
            template: String::new(),
            padding: 0,
            hide_when_absent: false,
            color: None,
            bg: None,
        },
    };

    Config {
        schema_url: None,
        lines: vec![ConfigLine {
            separator: String::new(),
            segments: vec![Segment::Template(template_seg)],
        }],
        powerline: false,
        default_fg: None,
        default_bg: None,
    }
}

// ── render spans for one segment ─────────────────────────────────────────────

fn render_segment_spans(seg: &BuilderSegment, ctx: &RenderCtx) -> Vec<Span<'static>> {
    let cfg = segment_to_config(seg);
    let ansi_str = crate::config::render::render(&cfg, ctx);
    let lines = ansi_to_lines(&ansi_str);
    lines
        .into_iter()
        .next()
        .map(|l| l.spans)
        .unwrap_or_default()
}

// ── public render entry point ─────────────────────────────────────────────────

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let (border_color, title_text) = if app.focus == Focus::Top {
        (Color::Cyan, "▶ Preview ")
    } else {
        (Color::DarkGray, "  Preview ")
    };
    let title_style = if app.focus == Focus::Top {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let block = Block::bordered()
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title_text, title_style));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let ctx = fixture_ctx();
    let mut ratatui_lines: Vec<Line<'static>> = Vec::new();

    for (line_idx, builder_line) in app.builder.lines.iter().enumerate() {
        let is_active = line_idx == app.active_line;

        // Gutter: `> ` for active line; `  ` otherwise.
        let gutter_text = if is_active { "> " } else { "  " };
        let gutter_style = if is_active && app.focus == Focus::Top && app.cursor == Cursor::Gutter {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };

        let mut spans: Vec<Span<'static>> = vec![Span::styled(gutter_text, gutter_style)];

        for (seg_idx, seg) in builder_line.segments.iter().enumerate() {
            // Separator between segments (skip before the first segment).
            if seg_idx > 0 {
                spans.push(Span::raw(builder_line.separator.clone()));
            }

            let mut seg_spans = render_segment_spans(seg, &ctx);

            let is_custom = matches!(seg, BuilderSegment::Custom { .. });
            let is_cursor =
                is_active && app.focus == Focus::Top && app.cursor == Cursor::Segment(seg_idx);

            for span in &mut seg_spans {
                if is_custom {
                    span.style = span.style.add_modifier(Modifier::DIM);
                }
                if is_cursor {
                    span.style = span.style.add_modifier(Modifier::REVERSED);
                }
            }

            spans.extend(seg_spans);
        }

        ratatui_lines.push(Line::from(spans));
    }

    // Virtual `+ new line` row when < MAX_LINES.
    if app.builder.lines.len() < crate::config::schema::MAX_LINES {
        let virtual_style = if app.focus == Focus::Top && app.cursor == Cursor::VirtualNewLine {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM)
        };
        ratatui_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("+ new line", virtual_style),
        ]));
    }

    let para = Paragraph::new(ratatui_lines);
    frame.render_widget(para, inner);
}

#[cfg(test)]
#[path = "top_tests.rs"]
mod tests;

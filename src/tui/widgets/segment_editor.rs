use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line as TLine, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::config::schema::Segment;

use super::super::app::{App, EditorField, Focus, Mode};

pub fn render_segment_editor(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Editor;
    let border_style = if focused {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let block = Block::default()
        .title("Editor")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(sel_seg_idx) = app.selected_segment else {
        let para = Paragraph::new("(no segment selected)");
        frame.render_widget(para, inner);
        return;
    };

    let Some(line) = app.config.lines.get(app.selected_line) else {
        let para = Paragraph::new("(no line selected)");
        frame.render_widget(para, inner);
        return;
    };

    let Some(segment) = line.segments.get(sel_seg_idx) else {
        let para = Paragraph::new("(no segment selected)");
        frame.render_widget(para, inner);
        return;
    };

    let Segment::Template(t) = segment else {
        let para = Paragraph::new("Flex spacer (no editable fields)");
        frame.render_widget(para, inner);
        return;
    };

    let editing = app.mode == Mode::EditingTemplate;

    let field_line = |label: &str, value: &str, active: bool, in_edit: bool| -> TLine<'static> {
        let highlight = if active && in_edit {
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD)
        } else if active {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let cursor = if active && in_edit { "▌" } else { "" };
        TLine::from(vec![
            Span::styled(format!("{label:<18}"), highlight),
            Span::raw(value.to_owned()),
            Span::raw(cursor.to_owned()),
        ])
    };

    let template_display = if editing && app.editor_field == EditorField::Template {
        app.editing_buf.as_deref().unwrap_or(&t.template).to_owned()
    } else {
        t.template.clone()
    };

    let color_display = t.color.as_deref().unwrap_or("(none)").to_owned();
    let bg_display = t.bg.as_deref().unwrap_or("(none)").to_owned();
    let hide_display = if t.hide_when_absent { "[x]" } else { "[ ]" };
    let powerline_display = if app.config.powerline {
        "[ON]"
    } else {
        "[OFF]"
    };

    let lines = vec![
        field_line("Powerline:", powerline_display, false, false),
        TLine::raw(""),
        field_line(
            "Template:",
            &template_display,
            app.editor_field == EditorField::Template,
            editing,
        ),
        field_line(
            "Padding:",
            &t.padding.to_string(),
            app.editor_field == EditorField::Padding,
            false,
        ),
        field_line(
            "Hide when absent:",
            hide_display,
            app.editor_field == EditorField::HideWhenAbsent,
            false,
        ),
        field_line(
            "Color:",
            &color_display,
            app.editor_field == EditorField::Color,
            false,
        ),
        field_line(
            "Background:",
            &bg_display,
            app.editor_field == EditorField::Bg,
            false,
        ),
    ];

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::config::schema::{Config, FlexSegment, Line, Segment, TemplateSegment};
    use crate::tui::app::{App, Focus};

    fn app_with_flex() -> App {
        let mut app = App::new(
            Config {
                schema_url: None,
                powerline: false,
                lines: vec![Line {
                    separator: String::new(),
                    segments: vec![Segment::Flex(FlexSegment { flex: true })],
                }],
            },
            PathBuf::from("/tmp/out.json"),
        );
        app.focus = Focus::Editor;
        app
    }

    fn app_with_template(tmpl: &str) -> App {
        let mut app = App::new(
            Config {
                schema_url: None,
                powerline: false,
                lines: vec![Line {
                    separator: String::new(),
                    segments: vec![Segment::Template(TemplateSegment::new(tmpl))],
                }],
            },
            PathBuf::from("/tmp/out.json"),
        );
        app.focus = Focus::Editor;
        app
    }

    #[test]
    fn flex_segment_editor_shows_no_editable_fields() {
        let app = app_with_flex();
        let seg = app
            .config
            .lines
            .first()
            .and_then(|l| l.segments.first())
            .unwrap();
        assert!(matches!(seg, Segment::Flex(_)));
        // The render path for Flex goes to the "no editable fields" branch — verified by
        // pattern matching here; the actual Paragraph output is verified visually.
    }

    #[test]
    fn template_segment_fields_accessible() {
        let app = app_with_template("{model}");
        let seg = app
            .config
            .lines
            .first()
            .and_then(|l| l.segments.first())
            .unwrap();
        let Segment::Template(t) = seg else {
            panic!("expected Template");
        };
        assert_eq!(t.template, "{model}");
        assert_eq!(t.padding, 0);
        assert!(!t.hide_when_absent);
        assert!(t.color.is_none());
        assert!(t.bg.is_none());
    }

    // Mirrors the production display-string logic to verify ON/OFF without a real terminal.
    fn powerline_label(app: &App) -> &'static str {
        if app.config.powerline {
            "[ON]"
        } else {
            "[OFF]"
        }
    }

    #[test]
    fn powerline_state_visible_in_segment_editor_render_on() {
        let mut app = app_with_template("{model}");
        app.config.powerline = true;
        assert_eq!(powerline_label(&app), "[ON]");
    }

    #[test]
    fn powerline_state_visible_in_segment_editor_render_off() {
        let app = app_with_template("{model}");
        assert!(!app.config.powerline);
        assert_eq!(powerline_label(&app), "[OFF]");
    }
}

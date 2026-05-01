use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::config::schema::Segment;

use super::super::app::{App, Focus};

pub fn render_segment_list(frame: &mut Frame, area: Rect, app: &App) {
    let segments = app
        .config
        .lines
        .get(app.selected_line)
        .map(|l| l.segments.as_slice())
        .unwrap_or(&[]);

    let mut items: Vec<ListItem> = segments
        .iter()
        .enumerate()
        .map(|(i, seg)| {
            let label = match seg {
                Segment::Template(t) => {
                    let name = first_placeholder_name(&t.template);
                    format!(
                        "{i}: {name} (pad={}, hide={})",
                        t.padding, t.hide_when_absent
                    )
                }
                Segment::Flex(_) => format!("{i}: <flex>"),
            };
            ListItem::new(label)
        })
        .collect();
    items.push(ListItem::new("+ add segment"));

    let focused = app.focus == Focus::SegmentList;
    let border_style = if focused {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title("Segments")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    // selected_segment indexes into the segment list; "+ add segment" is at segments.len()
    let sel = app.selected_segment.unwrap_or(segments.len());
    state.select(Some(sel));

    frame.render_stateful_widget(list, area, &mut state);
}

/// Returns the first `{...}` placeholder name in a template, or the whole template as fallback.
fn first_placeholder_name(template: &str) -> String {
    if let Some(start) = template.find('{') {
        if let Some(end) = template[start..].find('}') {
            return template[start..start + end + 1].to_owned();
        }
    }
    template.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_placeholder_extracts_name() {
        assert_eq!(first_placeholder_name("{model}"), "{model}");
        assert_eq!(first_placeholder_name("5h:{five_left}%"), "{five_left}");
    }

    #[test]
    fn first_placeholder_no_braces_returns_template() {
        assert_eq!(first_placeholder_name("hello"), "hello");
    }

    #[test]
    fn first_placeholder_unclosed_brace_returns_template() {
        assert_eq!(first_placeholder_name("{unclosed"), "{unclosed");
    }
}

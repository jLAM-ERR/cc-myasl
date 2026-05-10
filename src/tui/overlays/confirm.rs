use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style},
    text::{Line as TuiLine, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmKind {
    DeleteLine { segments: usize },
    QuitDirty,
}

/// Render a centered modal confirmation dialog.
pub fn render(frame: &mut Frame, area: Rect, kind: &ConfirmKind) {
    let popup = centered_rect(50, 20, area);
    frame.render_widget(Clear, popup);

    let block = Block::default().title(" Confirm ").borders(Borders::ALL);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = ratatui::layout::Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);

    let prompt = match kind {
        ConfirmKind::DeleteLine { segments } => {
            format!("Delete line with {} segment(s)?", segments)
        }
        ConfirmKind::QuitDirty => "Unsaved changes \u{2014} quit anyway?".to_owned(),
    };

    let prompt_line = TuiLine::from(Span::styled(
        prompt,
        Style::default().add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(
        Paragraph::new(prompt_line).alignment(Alignment::Center),
        rows[0],
    );

    // blank row
    frame.render_widget(Paragraph::new(""), rows[1]);

    let hint = TuiLine::from("  [y] yes    [n] no  ");
    frame.render_widget(Paragraph::new(hint).alignment(Alignment::Center), rows[2]);
}

/// Returns `Some(true)` on `y`/`Y`, `Some(false)` on `n`/`N`/`Esc`, `None` on other keys.
pub fn handle(event: KeyEvent) -> Option<bool> {
    match event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => Some(true),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => Some(false),
        _ => None,
    }
}

fn centered_rect(pct_w: u16, pct_h: u16, area: Rect) -> Rect {
    let vertical = ratatui::layout::Layout::vertical([
        Constraint::Percentage((100 - pct_h) / 2),
        Constraint::Percentage(pct_h),
        Constraint::Percentage((100 - pct_h) / 2),
    ])
    .split(area);
    ratatui::layout::Layout::horizontal([
        Constraint::Percentage((100 - pct_w) / 2),
        Constraint::Percentage(pct_w),
        Constraint::Percentage((100 - pct_w) / 2),
    ])
    .split(vertical[1])[1]
}

//! KeyEvent → App method dispatcher for Phase 4.
//!
//! Kept separate from app.rs to stay within the 500-LOC invariant.
//! Declared as `pub mod app_handle` in tui/mod.rs.

use crate::tui::app::{App, Cursor, Focus, Mode};
use crate::tui::builder::BuilderSegment;

impl App {
    /// Top-level event dispatcher — called once per key press in the run loop.
    pub fn handle(&mut self, event: crossterm::event::KeyEvent) {
        match self.mode {
            Mode::Browsing => self.handle_browsing(event),
            Mode::Filter => self.handle_filter(event),
            Mode::EditingSeparator => self.handle_editing_separator(event),
            Mode::PickingFgColor | Mode::PickingBgColor => self.handle_picking_color(event),
            Mode::ConfirmDelete => self.handle_confirm_delete(event),
            Mode::ConfirmQuit => self.handle_confirm_quit(event),
            Mode::Help => {
                self.mode = Mode::Browsing;
            }
            Mode::Saving => {} // no input during save (synchronous in run)
        }
    }

    fn handle_browsing(&mut self, event: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};
        match (event.code, event.modifiers) {
            (KeyCode::Tab, KeyModifiers::NONE) => self.focus_cycle_forward(),
            (KeyCode::BackTab, _) => self.focus_cycle_backward(),
            (KeyCode::Char('q'), KeyModifiers::NONE) => self.request_quit(),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => self.should_quit = true,
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => self.mode = Mode::Saving,
            (KeyCode::Char('?'), KeyModifiers::NONE) => self.mode = Mode::Help,
            _ => match self.focus {
                Focus::Top => self.handle_browsing_top(event),
                Focus::Middle => self.handle_browsing_middle(event),
                Focus::Bottom => {} // read-only
            },
        }
    }

    fn handle_browsing_top(&mut self, event: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};
        match (event.code, event.modifiers) {
            (KeyCode::Left, _) | (KeyCode::Char('h'), KeyModifiers::NONE) => self.cursor_left(),
            (KeyCode::Right, _) | (KeyCode::Char('l'), KeyModifiers::NONE) => self.cursor_right(),
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => self.cursor_up_line(),
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                self.cursor_down_line()
            }
            (KeyCode::Char('<'), KeyModifiers::NONE) => {
                if matches!(self.cursor, Cursor::Segment(_)) {
                    self.reorder_left();
                }
            }
            (KeyCode::Char('>'), KeyModifiers::NONE) => {
                if matches!(self.cursor, Cursor::Segment(_)) {
                    self.reorder_right();
                }
            }
            (KeyCode::Char('x'), KeyModifiers::NONE) => match self.cursor {
                Cursor::Segment(_) => self.delete_segment(),
                Cursor::Gutter => self.delete_line(),
                Cursor::VirtualNewLine => {}
            },
            (KeyCode::Char('s'), KeyModifiers::NONE) => {
                if matches!(self.cursor, Cursor::Gutter) {
                    self.mode = Mode::EditingSeparator;
                    // pre-fill buffer with current separator
                    let sep = self
                        .builder
                        .lines
                        .get(self.active_line)
                        .map(|l| l.separator.clone())
                        .unwrap_or_default();
                    self.picker_filter = sep;
                }
            }
            (KeyCode::Char('J'), KeyModifiers::SHIFT) => {
                if matches!(self.cursor, Cursor::Gutter) {
                    self.move_line_down();
                }
            }
            (KeyCode::Char('K'), KeyModifiers::SHIFT) => {
                if matches!(self.cursor, Cursor::Gutter) {
                    self.move_line_up();
                }
            }
            (KeyCode::Char('y'), KeyModifiers::NONE) => {
                if matches!(self.cursor, Cursor::Gutter) {
                    self.duplicate_line();
                }
            }
            (KeyCode::Char('c'), KeyModifiers::NONE) => {
                if matches!(self.cursor, Cursor::Segment(_)) {
                    self.mode = Mode::PickingFgColor;
                }
            }
            (KeyCode::Char('b'), KeyModifiers::NONE) => {
                if matches!(self.cursor, Cursor::Segment(_)) {
                    self.mode = Mode::PickingBgColor;
                }
            }
            (KeyCode::Enter, _) => {
                if matches!(self.cursor, Cursor::VirtualNewLine) {
                    self.add_line();
                }
            }
            (KeyCode::Char('1'), KeyModifiers::NONE) => self.set_active_line(0),
            (KeyCode::Char('2'), KeyModifiers::NONE) => self.set_active_line(1),
            (KeyCode::Char('3'), KeyModifiers::NONE) => self.set_active_line(2),
            _ => {}
        }
    }

    fn handle_browsing_middle(&mut self, event: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};
        match (event.code, event.modifiers) {
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => self.picker_select_up(),
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                self.picker_select_down()
            }
            (KeyCode::Char('['), KeyModifiers::NONE) => self.tab_cycle_prev(),
            (KeyCode::Char(']'), KeyModifiers::NONE) => self.tab_cycle_next(),
            (KeyCode::Char(' '), KeyModifiers::NONE) => self.toggle_selected_preset(),
            (KeyCode::Char('/'), KeyModifiers::NONE) => self.open_filter(),
            _ => {}
        }
    }

    fn handle_filter(&mut self, event: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};
        match (event.code, event.modifiers) {
            (KeyCode::Esc, _) => self.cancel_filter(),
            (KeyCode::Enter, _) => self.commit_filter(),
            (KeyCode::Backspace, _) => self.filter_backspace(),
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                self.filter_type(c)
            }
            _ => {}
        }
    }

    fn handle_editing_separator(&mut self, event: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};
        match (event.code, event.modifiers) {
            (KeyCode::Esc, _) => {
                self.picker_filter.clear();
                self.mode = Mode::Browsing;
            }
            (KeyCode::Enter, _) => {
                let sep = self.picker_filter.clone();
                self.picker_filter.clear();
                self.set_separator(self.active_line, sep);
                self.mode = Mode::Browsing;
            }
            (KeyCode::Backspace, _) => {
                self.picker_filter.pop();
            }
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                self.picker_filter.push(c);
            }
            _ => {}
        }
    }

    fn handle_picking_color(&mut self, event: crossterm::event::KeyEvent) {
        use crate::tui::overlays::color_picker::{PickerEvent, handle as pick_handle};
        let ev = pick_handle(event, &mut self.color_picker_selected);
        let is_fg = self.mode == Mode::PickingFgColor;
        match ev {
            PickerEvent::Commit(c) => {
                if is_fg {
                    self.apply_fg_color(Some(c));
                } else {
                    self.apply_bg_color(Some(c));
                }
                self.mode = Mode::Browsing;
            }
            PickerEvent::CommitNone => {
                if is_fg {
                    self.apply_fg_color(None);
                } else {
                    self.apply_bg_color(None);
                }
                self.mode = Mode::Browsing;
            }
            PickerEvent::Cancel => {
                self.mode = Mode::Browsing;
            }
            PickerEvent::Pending => {}
        }
    }

    fn handle_confirm_delete(&mut self, event: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match event.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => self.confirm_delete_yes(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => self.confirm_delete_no(),
            _ => {}
        }
    }

    fn handle_confirm_quit(&mut self, event: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match event.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => self.confirm_quit_yes(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => self.confirm_quit_no(),
            _ => {}
        }
    }

    fn apply_fg_color(&mut self, c: Option<crate::config::named_color::NamedColor>) {
        if let Cursor::Segment(i) = self.cursor {
            if let Some(seg) = self.builder.lines[self.active_line].segments.get_mut(i) {
                match seg {
                    BuilderSegment::Preset { color, .. } => *color = c,
                    BuilderSegment::Custom { color, .. } => *color = c,
                    BuilderSegment::FlexSpacer => {} // no color on spacer
                }
                self.dirty = true;
            }
        }
    }

    fn apply_bg_color(&mut self, c: Option<crate::config::named_color::NamedColor>) {
        if let Cursor::Segment(i) = self.cursor {
            if let Some(seg) = self.builder.lines[self.active_line].segments.get_mut(i) {
                match seg {
                    BuilderSegment::Preset { bg, .. } => *bg = c,
                    BuilderSegment::Custom { bg, .. } => *bg = c,
                    BuilderSegment::FlexSpacer => {} // no color on spacer
                }
                self.dirty = true;
            }
        }
    }

    pub fn picker_select_up(&mut self) {
        if self.picker_selected > 0 {
            self.picker_selected -= 1;
        }
    }

    pub fn picker_select_down(&mut self) {
        let count = self.visible_preset_count();
        if count > 0 && self.picker_selected + 1 < count {
            self.picker_selected += 1;
        }
    }

    fn visible_preset_count(&self) -> usize {
        use crate::tui::catalog::{Category, by_category};
        if self.active_tab == Category::Appearance {
            return 3 + self.builder.lines.len();
        }
        let filter = self.picker_filter.to_lowercase();
        by_category(self.active_tab)
            .filter(|p| {
                if filter.is_empty() {
                    return true;
                }
                p.label.to_lowercase().contains(&filter)
                    || p.template.to_lowercase().contains(&filter)
            })
            .count()
    }

    /// Jump active_line to `idx` if in bounds; emit status if out of bounds.
    pub fn set_active_line(&mut self, idx: usize) {
        if idx < self.builder.lines.len() {
            self.active_line = idx;
            self.cursor = Cursor::Gutter;
        } else {
            self.set_status(&format!("no line {}", idx + 1));
        }
    }

    /// Toggle the preset at `picker_selected` in current tab (no-op for Appearance).
    pub fn toggle_selected_preset(&mut self) {
        use crate::tui::catalog::Category;
        if self.active_tab == Category::Appearance {
            return;
        }
        let idx = self.picker_selected;
        self.toggle_preset(self.active_tab, idx);
    }

    pub fn set_status_ok(&mut self, msg: String) {
        use crate::time::now_unix;
        self.status_message = Some((msg, now_unix() + 2));
    }

    pub fn set_status_err(&mut self, msg: String) {
        use crate::time::now_unix;
        self.status_message = Some((msg, now_unix() + 5));
    }
}

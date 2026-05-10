//! Phase 4 App state machine — pure methods, no crossterm/ratatui imports.
//!
//! `KeyEvent → method call` translation lives in the draw/run loop (Task 11).
//! All methods are infallible and return nothing; callers inspect fields directly.

use std::path::PathBuf;

use crate::config::schema::{Config, MAX_LINES, ValidationError};
use crate::time::now_unix;
use crate::tui::builder::{BuilderLine, BuilderSegment, BuilderState, from_config};
use crate::tui::catalog::{Category, by_category};

/// Status message lifetime in seconds.
const STATUS_TTL: u64 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Browsing,
    Filter,
    EditingSeparator,
    PickingFgColor,
    PickingBgColor,
    Saving,
    Help,
    ConfirmDelete,
    ConfirmQuit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    Gutter,
    Segment(usize),
    VirtualNewLine,
}

pub struct App {
    pub builder: BuilderState,
    pub output_path: PathBuf,
    pub active_line: usize,
    pub cursor: Cursor,
    pub focus: Focus,
    pub mode: Mode,
    pub active_tab: Category,
    pub picker_filter: String,
    pub picker_selected: usize,
    pub color_picker_selected: usize,
    pub dirty: bool,
    pub status_message: Option<(String, u64)>,
    pub last_save_errors: Vec<ValidationError>,
    pub should_quit: bool,
}

impl App {
    pub fn new(config: Config, output_path: PathBuf) -> App {
        let builder = from_config(&config);
        App {
            builder,
            output_path,
            active_line: 0,
            cursor: Cursor::Gutter,
            focus: Focus::Top,
            mode: Mode::Browsing,
            active_tab: Category::Workspace,
            picker_filter: String::new(),
            picker_selected: 0,
            color_picker_selected: 0,
            dirty: false,
            status_message: None,
            last_save_errors: Vec::new(),
            should_quit: false,
        }
    }

    // ── cursor walks ──────────────────────────────────────────────────────────

    pub fn cursor_left(&mut self) {
        match self.cursor {
            Cursor::Segment(0) => self.cursor = Cursor::Gutter,
            Cursor::Segment(i) => self.cursor = Cursor::Segment(i - 1),
            Cursor::Gutter | Cursor::VirtualNewLine => {}
        }
    }

    pub fn cursor_right(&mut self) {
        let seg_count = self.active_line_seg_count();
        match self.cursor {
            Cursor::Gutter => {
                if seg_count > 0 {
                    self.cursor = Cursor::Segment(0);
                }
            }
            Cursor::Segment(i) => {
                if i + 1 < seg_count {
                    self.cursor = Cursor::Segment(i + 1);
                }
            }
            Cursor::VirtualNewLine => {}
        }
    }

    pub fn cursor_up_line(&mut self) {
        match self.cursor {
            Cursor::VirtualNewLine => {
                let last = self.builder.lines.len().saturating_sub(1);
                self.active_line = last;
                self.cursor = Cursor::Gutter;
            }
            _ => {
                if self.active_line > 0 {
                    self.active_line -= 1;
                    self.cursor = Cursor::Gutter;
                }
            }
        }
    }

    pub fn cursor_down_line(&mut self) {
        match self.cursor {
            Cursor::VirtualNewLine => {}
            _ => {
                let n = self.builder.lines.len();
                if self.active_line + 1 < n {
                    self.active_line += 1;
                    self.cursor = Cursor::Gutter;
                } else if n < MAX_LINES {
                    self.cursor = Cursor::VirtualNewLine;
                }
                // else: at last real line and already at MAX → no-op
            }
        }
    }

    // ── focus cycle ───────────────────────────────────────────────────────────

    pub fn focus_cycle_forward(&mut self) {
        self.focus = match self.focus {
            Focus::Top => Focus::Middle,
            Focus::Middle => Focus::Bottom,
            Focus::Bottom => Focus::Top,
        };
    }

    pub fn focus_cycle_backward(&mut self) {
        self.focus = match self.focus {
            Focus::Top => Focus::Bottom,
            Focus::Middle => Focus::Top,
            Focus::Bottom => Focus::Middle,
        };
    }

    // ── tab cycle ─────────────────────────────────────────────────────────────

    pub fn tab_cycle_next(&mut self) {
        self.mode = Mode::Browsing;
        let cats = Category::ordered();
        let pos = cats.iter().position(|c| *c == self.active_tab).unwrap_or(0);
        self.active_tab = cats[(pos + 1) % cats.len()];
        // Category change clears filter (per plan decision).
        self.picker_filter.clear();
        self.picker_selected = 0;
    }

    pub fn tab_cycle_prev(&mut self) {
        self.mode = Mode::Browsing;
        let cats = Category::ordered();
        let pos = cats.iter().position(|c| *c == self.active_tab).unwrap_or(0);
        self.active_tab = cats[(pos + cats.len() - 1) % cats.len()];
        // Category change clears filter (per plan decision).
        self.picker_filter.clear();
        self.picker_selected = 0;
    }

    // ── line ops ──────────────────────────────────────────────────────────────

    pub fn add_line(&mut self) {
        if self.builder.lines.len() >= MAX_LINES {
            self.set_status("max 3 lines");
            return;
        }
        self.builder.lines.push(BuilderLine {
            separator: " | ".into(),
            segments: vec![],
        });
        self.active_line = self.builder.lines.len() - 1;
        self.cursor = Cursor::Gutter;
        self.dirty = true;
    }

    /// Delete active line.  If line has ≥ 1 segment, enters `ConfirmDelete`.
    /// If line has 0 segments, deletes immediately.
    /// If only 1 line remains, sets status and no-ops.
    pub fn delete_line(&mut self) {
        if self.builder.lines.len() == 1 {
            self.set_status("cannot remove last line");
            return;
        }
        let seg_count = self.active_line_seg_count();
        if seg_count >= 1 {
            self.mode = Mode::ConfirmDelete;
        } else {
            self.do_delete_line();
        }
    }

    /// Called by the confirm-yes handler.
    pub fn confirm_delete_yes(&mut self) {
        self.mode = Mode::Browsing;
        self.do_delete_line();
    }

    pub fn confirm_delete_no(&mut self) {
        self.mode = Mode::Browsing;
    }

    fn do_delete_line(&mut self) {
        self.builder.lines.remove(self.active_line);
        if self.active_line >= self.builder.lines.len() {
            self.active_line = self.builder.lines.len().saturating_sub(1);
        }
        self.cursor = Cursor::Gutter;
        self.dirty = true;
    }

    pub fn move_line_up(&mut self) {
        if self.active_line > 0 {
            self.builder
                .lines
                .swap(self.active_line, self.active_line - 1);
            self.active_line -= 1;
            self.dirty = true;
        }
    }

    pub fn move_line_down(&mut self) {
        let n = self.builder.lines.len();
        if self.active_line + 1 < n {
            self.builder
                .lines
                .swap(self.active_line, self.active_line + 1);
            self.active_line += 1;
            self.dirty = true;
        }
    }

    pub fn duplicate_line(&mut self) {
        if self.builder.lines.len() >= MAX_LINES {
            self.set_status("max 3 lines");
            return;
        }
        let dup = self.builder.lines[self.active_line].clone();
        self.builder.lines.insert(self.active_line + 1, dup);
        self.active_line += 1;
        self.cursor = Cursor::Gutter;
        self.dirty = true;
    }

    pub fn edit_separator(&mut self) {
        self.mode = Mode::EditingSeparator;
    }

    // ── segment ops ───────────────────────────────────────────────────────────

    pub fn delete_segment(&mut self) {
        if let Cursor::Segment(i) = self.cursor {
            let line = &mut self.builder.lines[self.active_line];
            if i < line.segments.len() {
                line.segments.remove(i);
                let new_len = line.segments.len();
                self.cursor = if new_len == 0 {
                    Cursor::Gutter
                } else if i >= new_len {
                    Cursor::Segment(new_len - 1)
                } else {
                    Cursor::Segment(i)
                };
                self.dirty = true;
            }
        }
    }

    pub fn reorder_left(&mut self) {
        if let Cursor::Segment(i) = self.cursor {
            if i > 0 {
                self.builder.lines[self.active_line].segments.swap(i, i - 1);
                self.cursor = Cursor::Segment(i - 1);
                self.dirty = true;
            }
        }
    }

    pub fn reorder_right(&mut self) {
        if let Cursor::Segment(i) = self.cursor {
            let len = self.builder.lines[self.active_line].segments.len();
            if i + 1 < len {
                self.builder.lines[self.active_line].segments.swap(i, i + 1);
                self.cursor = Cursor::Segment(i + 1);
                self.dirty = true;
            }
        }
    }

    /// Toggle a preset on/off for the active line.
    ///
    /// Custom-segment protection: if a `BuilderSegment::Custom` whose template
    /// matches the preset's template exists, the preset is added as a new
    /// segment regardless (no removal of the custom entry).
    pub fn toggle_preset(&mut self, category: Category, preset_index: usize) {
        let preset = match by_category(category).nth(preset_index) {
            Some(p) => p,
            None => return,
        };
        let line = &mut self.builder.lines[self.active_line];

        // Check if a matching Preset segment (by id) exists.
        let existing_preset_pos = line
            .segments
            .iter()
            .position(|s| matches!(s, BuilderSegment::Preset { id, .. } if *id == preset.id));

        if let Some(pos) = existing_preset_pos {
            // Toggle off — remove the preset segment.
            line.segments.remove(pos);
            // Adjust cursor if needed.
            if let Cursor::Segment(ci) = self.cursor {
                let new_len = line.segments.len();
                self.cursor = if new_len == 0 {
                    Cursor::Gutter
                } else if ci >= new_len {
                    Cursor::Segment(new_len - 1)
                } else {
                    Cursor::Segment(ci)
                };
            }
        } else {
            // Toggle on — add preset (even if a Custom with same template exists).
            line.segments.push(BuilderSegment::Preset {
                id: preset.id,
                color: preset.default_color,
                bg: preset.default_bg,
            });
        }
        self.dirty = true;
    }

    // ── filter mode ──────────────────────────────────────────────────────────

    /// Open filter mode. If already in `Mode::Filter`, clear the input but stay
    /// in Filter mode. If in `Mode::Browsing` with or without a committed filter,
    /// transition to `Mode::Filter` with an empty input. The previous filter (if
    /// any) is discarded.
    pub fn open_filter(&mut self) {
        if self.mode == Mode::Filter {
            // Already filtering — `/` again clears and re-opens.
            self.picker_filter.clear();
        } else {
            self.mode = Mode::Filter;
            self.picker_filter.clear();
            self.picker_selected = 0;
        }
    }

    /// Esc in filter mode: clear filter and return to Browsing.
    pub fn cancel_filter(&mut self) {
        self.picker_filter.clear();
        self.mode = Mode::Browsing;
    }

    /// Enter in filter mode: leave filter mode but keep filter active.
    pub fn commit_filter(&mut self) {
        self.mode = Mode::Browsing;
        // picker_filter retained intentionally.
    }

    /// Append a char to the filter input (resets cursor to first visible row).
    pub fn filter_type(&mut self, c: char) {
        self.picker_filter.push(c);
        self.picker_selected = 0;
    }

    /// Backspace in the filter input (resets cursor to first visible row).
    pub fn filter_backspace(&mut self) {
        self.picker_filter.pop();
        self.picker_selected = 0;
    }

    /// Clear an active committed filter (e.g. from the clear-hint action).
    pub fn clear_filter(&mut self) {
        self.picker_filter.clear();
        self.picker_selected = 0;
    }

    // ── appearance settings mutations ────────────────────────────────────────

    pub fn toggle_powerline(&mut self) {
        self.builder.powerline = !self.builder.powerline;
        self.dirty = true;
    }

    pub fn set_default_fg(&mut self, c: Option<crate::config::named_color::NamedColor>) {
        self.builder.default_fg = c;
        self.dirty = true;
    }

    pub fn set_default_bg(&mut self, c: Option<crate::config::named_color::NamedColor>) {
        self.builder.default_bg = c;
        self.dirty = true;
    }

    pub fn set_separator(&mut self, line_idx: usize, sep: String) {
        if let Some(line) = self.builder.lines.get_mut(line_idx) {
            line.separator = sep;
            self.dirty = true;
        }
    }

    // ── confirm quit ──────────────────────────────────────────────────────────

    pub fn request_quit(&mut self) {
        if self.dirty {
            self.mode = Mode::ConfirmQuit;
        } else {
            self.should_quit = true;
        }
    }

    pub fn confirm_quit_yes(&mut self) {
        self.mode = Mode::Browsing;
        self.should_quit = true;
    }

    pub fn confirm_quit_no(&mut self) {
        self.mode = Mode::Browsing;
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    fn active_line_seg_count(&self) -> usize {
        self.builder
            .lines
            .get(self.active_line)
            .map(|l| l.segments.len())
            .unwrap_or(0)
    }

    pub(crate) fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_owned(), now_unix() + STATUS_TTL));
    }
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "app_tests_b.rs"]
mod tests_b;

#[cfg(test)]
#[path = "filter_tests.rs"]
mod filter_tests;

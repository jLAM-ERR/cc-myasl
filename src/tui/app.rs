use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::schema::{Config, Line, MAX_LINES, Segment, TemplateSegment, ValidationError};

use super::app_color::handle_picking_color;
use super::app_editor::{
    editor_padding_decrement, editor_padding_increment, editor_toggle_hide, handle_editing_template,
};
use super::app_picker::handle_picking_placeholder;
use super::app_save::{handle_confirm_quit, handle_ctrl_s};

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Browsing,
    EditingTemplate,
    PickingPlaceholder,
    PickingFgColor,
    PickingBgColor,
    Saving,
    Help,
    ConfirmQuit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    LineList,
    SegmentList,
    Editor,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditorField {
    Template,
    Padding,
    HideWhenAbsent,
    Color,
    Bg,
}

impl EditorField {
    fn next(&self) -> Self {
        match self {
            Self::Template => Self::Padding,
            Self::Padding => Self::HideWhenAbsent,
            Self::HideWhenAbsent => Self::Color,
            Self::Color => Self::Bg,
            Self::Bg => Self::Template,
        }
    }
}

pub struct App {
    pub config: Config,
    pub output_path: PathBuf,
    pub mode: Mode,
    pub should_quit: bool,
    pub dirty: bool,
    pub selected_line: usize,
    pub selected_segment: Option<usize>,
    pub focus: Focus,
    pub editor_field: EditorField,
    /// Temporary buffer while in EditingTemplate mode; None outside that mode.
    pub editing_buf: Option<String>,
    /// Current filter string in PickingPlaceholder mode.
    pub picker_filter: String,
    /// Currently highlighted row in PickingPlaceholder mode.
    pub picker_selected: usize,
    /// Currently highlighted row in PickingFgColor / PickingBgColor mode (0..=8).
    pub color_picker_selected: usize,
    /// Validation errors from the most recent failed save attempt.
    pub last_save_errors: Vec<ValidationError>,
    /// Transient status message and the unix timestamp when it was set.
    pub status_message: Option<(String, u64)>,
}

impl App {
    pub fn new(config: Config, output_path: PathBuf) -> Self {
        let selected_segment = config
            .lines
            .first()
            .and_then(|l| if l.segments.is_empty() { None } else { Some(0) });
        App {
            config,
            output_path,
            mode: Mode::Browsing,
            should_quit: false,
            dirty: false,
            selected_line: 0,
            selected_segment,
            focus: Focus::LineList,
            editor_field: EditorField::Template,
            editing_buf: None,
            picker_filter: String::new(),
            picker_selected: 0,
            color_picker_selected: 0,
            last_save_errors: Vec::new(),
            status_message: None,
        }
    }

    pub fn handle(&mut self, event: KeyEvent) {
        match self.mode {
            Mode::Browsing => self.handle_browsing(event),
            Mode::EditingTemplate => handle_editing_template(self, event),
            Mode::PickingPlaceholder => handle_picking_placeholder(self, event),
            Mode::PickingFgColor | Mode::PickingBgColor => handle_picking_color(self, event),
            Mode::ConfirmQuit => handle_confirm_quit(self, event),
            // Any key dismisses the help overlay.
            Mode::Help => self.mode = Mode::Browsing,
            _ => {}
        }
    }

    fn handle_browsing(&mut self, event: KeyEvent) {
        let code = event.code;
        let mods = event.modifiers;

        match (code, mods) {
            (KeyCode::Char('q'), KeyModifiers::NONE) => {
                if self.dirty {
                    self.mode = Mode::ConfirmQuit;
                } else {
                    self.should_quit = true;
                }
            }
            (KeyCode::Tab, KeyModifiers::NONE) => {
                if self.focus == Focus::Editor {
                    self.editor_field = self.editor_field.next();
                } else {
                    self.focus = match self.focus {
                        Focus::LineList => Focus::SegmentList,
                        Focus::SegmentList => Focus::Editor,
                        Focus::Editor => Focus::LineList,
                    };
                }
            }
            (KeyCode::Esc, _) if self.focus == Focus::Editor => {
                self.focus = Focus::SegmentList;
            }
            (KeyCode::Char('j'), KeyModifiers::NONE) | (KeyCode::Down, _) => {
                self.move_down();
            }
            (KeyCode::Char('k'), KeyModifiers::NONE) | (KeyCode::Up, _) => {
                self.move_up();
            }
            // Shift+J: reorder segment down
            (KeyCode::Char('J'), KeyModifiers::SHIFT) => {
                if self.focus == Focus::SegmentList {
                    self.reorder_segment_down();
                }
            }
            // Shift+K: reorder segment up
            (KeyCode::Char('K'), KeyModifiers::SHIFT) => {
                if self.focus == Focus::SegmentList {
                    self.reorder_segment_up();
                }
            }
            // 'n': add new line (LineList only)
            (KeyCode::Char('n'), KeyModifiers::NONE) => {
                if self.focus == Focus::LineList {
                    self.add_line();
                }
            }
            // 'D': delete line (LineList only)
            (KeyCode::Char('D'), KeyModifiers::SHIFT) => {
                if self.focus == Focus::LineList {
                    self.delete_line();
                }
            }
            // 'd': delete segment (SegmentList only)
            (KeyCode::Char('d'), KeyModifiers::NONE) => {
                if self.focus == Focus::SegmentList {
                    self.delete_segment();
                }
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                self.handle_enter();
            }
            // Editor-focus key handlers (only when Focus::Editor)
            (KeyCode::Char('+'), _) if self.focus == Focus::Editor => {
                editor_padding_increment(self);
            }
            (KeyCode::Char('-'), _) if self.focus == Focus::Editor => {
                editor_padding_decrement(self);
            }
            (KeyCode::Char(' '), _) if self.focus == Focus::Editor => {
                if self.editor_field == EditorField::HideWhenAbsent {
                    editor_toggle_hide(self);
                }
            }
            (KeyCode::Char('c'), _) if self.focus == Focus::Editor => {
                if self.editor_field == EditorField::Color {
                    self.color_picker_selected = 0;
                    self.mode = Mode::PickingFgColor;
                }
            }
            (KeyCode::Char('b'), _) if self.focus == Focus::Editor => {
                if self.editor_field == EditorField::Bg {
                    self.color_picker_selected = 0;
                    self.mode = Mode::PickingBgColor;
                }
            }
            // '?': show help overlay
            (KeyCode::Char('?'), _) => {
                self.mode = Mode::Help;
            }
            // Shift+P: toggle Powerline mode globally (any focus)
            (KeyCode::Char('P'), KeyModifiers::SHIFT) => {
                self.config.powerline = !self.config.powerline;
                self.dirty = true;
            }
            // Ctrl+S: save config
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                handle_ctrl_s(self);
            }
            _ => {}
        }
    }

    fn move_down(&mut self) {
        match self.focus {
            Focus::LineList => {
                let max = self.config.lines.len().saturating_sub(1);
                if self.selected_line < max {
                    self.selected_line += 1;
                    self.reset_segment_selection();
                }
            }
            Focus::SegmentList => {
                let seg_count = self.current_line_segments_len();
                if let Some(sel) = self.selected_segment {
                    if sel + 1 < seg_count {
                        self.selected_segment = Some(sel + 1);
                    }
                } else if seg_count > 0 {
                    self.selected_segment = Some(0);
                }
            }
            Focus::Editor => {}
        }
    }

    fn move_up(&mut self) {
        match self.focus {
            Focus::LineList => {
                if self.selected_line > 0 {
                    self.selected_line -= 1;
                    self.reset_segment_selection();
                }
            }
            Focus::SegmentList => {
                if let Some(sel) = self.selected_segment {
                    if sel > 0 {
                        self.selected_segment = Some(sel - 1);
                    }
                }
            }
            Focus::Editor => {}
        }
    }

    fn reset_segment_selection(&mut self) {
        self.selected_segment = self
            .config
            .lines
            .get(self.selected_line)
            .and_then(|l| if l.segments.is_empty() { None } else { Some(0) });
    }

    pub fn current_line_segments_len(&self) -> usize {
        self.config
            .lines
            .get(self.selected_line)
            .map(|l| l.segments.len())
            .unwrap_or(0)
    }

    fn add_line(&mut self) {
        if self.config.lines.len() < MAX_LINES {
            self.config.lines.push(Line {
                separator: String::new(),
                segments: vec![],
            });
            self.dirty = true;
        }
    }

    fn delete_line(&mut self) {
        if self.config.lines.len() > 1 {
            self.config.lines.remove(self.selected_line);
            if self.selected_line >= self.config.lines.len() {
                self.selected_line = self.config.lines.len().saturating_sub(1);
            }
            self.reset_segment_selection();
            self.dirty = true;
        }
    }

    fn delete_segment(&mut self) {
        if let Some(sel) = self.selected_segment {
            let seg_count = self.current_line_segments_len();
            if sel < seg_count {
                if let Some(line) = self.config.lines.get_mut(self.selected_line) {
                    line.segments.remove(sel);
                }
                let new_len = self.current_line_segments_len();
                self.selected_segment = if new_len == 0 {
                    None
                } else if sel >= new_len {
                    Some(new_len - 1)
                } else {
                    Some(sel)
                };
                self.dirty = true;
            }
        }
    }

    fn reorder_segment_down(&mut self) {
        if let Some(sel) = self.selected_segment {
            let seg_count = self.current_line_segments_len();
            if sel + 1 < seg_count {
                if let Some(line) = self.config.lines.get_mut(self.selected_line) {
                    line.segments.swap(sel, sel + 1);
                }
                self.selected_segment = Some(sel + 1);
                self.dirty = true;
            }
        }
    }

    fn reorder_segment_up(&mut self) {
        if let Some(sel) = self.selected_segment {
            if sel > 0 {
                if let Some(line) = self.config.lines.get_mut(self.selected_line) {
                    line.segments.swap(sel, sel - 1);
                }
                self.selected_segment = Some(sel - 1);
                self.dirty = true;
            }
        }
    }

    fn handle_enter(&mut self) {
        if self.focus == Focus::Editor {
            if self.editor_field == EditorField::Template {
                let li = self.selected_line;
                let current = self
                    .selected_segment
                    .and_then(|si| {
                        self.config
                            .lines
                            .get(li)
                            .and_then(|l| l.segments.get(si))
                            .and_then(|s| {
                                if let Segment::Template(t) = s {
                                    Some(t.template.clone())
                                } else {
                                    None
                                }
                            })
                    })
                    .unwrap_or_default();
                self.editing_buf = Some(current);
                self.mode = Mode::EditingTemplate;
            }
            return;
        }
        if self.focus != Focus::SegmentList {
            return;
        }
        let seg_count = self.current_line_segments_len();
        let sel = self.selected_segment.unwrap_or(seg_count);
        if sel < seg_count {
            let current = self
                .config
                .lines
                .get(self.selected_line)
                .and_then(|l| l.segments.get(sel))
                .and_then(|s| {
                    if let Segment::Template(t) = s {
                        Some(t.template.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            self.editing_buf = Some(current);
            self.focus = Focus::Editor;
            self.editor_field = EditorField::Template;
            self.mode = Mode::EditingTemplate;
        } else {
            // Enter on "+ add segment"
            let new_seg = Segment::Template(TemplateSegment {
                template: "{model}".into(),
                padding: 0,
                hide_when_absent: false,
                color: None,
                bg: None,
            });
            if let Some(line) = self.config.lines.get_mut(self.selected_line) {
                line.segments.push(new_seg);
            }
            self.selected_segment = Some(seg_count);
            self.editing_buf = Some("{model}".into());
            self.focus = Focus::Editor;
            self.editor_field = EditorField::Template;
            self.mode = Mode::EditingTemplate;
            self.dirty = true;
        }
    }
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "app_powerline_tests.rs"]
mod powerline_tests;

#[cfg(test)]
#[path = "app_help_tests.rs"]
mod help_tests;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::named_color::NamedColor;
use crate::config::schema::{Config, Line, Segment, TemplateSegment};

use super::color_picker::{ENTRY_COUNT, PickerEvent, handle as cp_handle};
use super::confirm::{ConfirmKind, handle as confirm_handle};
use super::help::handle as help_handle;
use super::save::save;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn minimal_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        default_fg: None,
        default_bg: None,
        lines: vec![Line {
            separator: String::new(),
            segments: vec![Segment::Template(TemplateSegment::new("{model}"))],
        }],
    }
}

// ── color_picker ─────────────────────────────────────────────────────────────

#[test]
fn color_picker_down_increments_selected() {
    let mut sel = 0usize;
    let ev = cp_handle(key(KeyCode::Down), &mut sel);
    assert_eq!(sel, 1);
    assert_eq!(ev, PickerEvent::Pending);
}

#[test]
fn color_picker_j_increments_selected() {
    let mut sel = 0usize;
    cp_handle(key(KeyCode::Char('j')), &mut sel);
    assert_eq!(sel, 1);
}

#[test]
fn color_picker_up_k_decrements_selected() {
    let mut sel = 3usize;
    cp_handle(key(KeyCode::Char('k')), &mut sel);
    assert_eq!(sel, 2);
    cp_handle(key(KeyCode::Up), &mut sel);
    assert_eq!(sel, 1);
}

#[test]
fn color_picker_down_wraps_at_last() {
    let mut sel = ENTRY_COUNT - 1;
    let ev = cp_handle(key(KeyCode::Down), &mut sel);
    assert_eq!(sel, 0, "wraps back to 0");
    assert_eq!(ev, PickerEvent::Pending);
}

#[test]
fn color_picker_up_wraps_at_zero() {
    let mut sel = 0usize;
    let ev = cp_handle(key(KeyCode::Up), &mut sel);
    assert_eq!(sel, ENTRY_COUNT - 1, "wraps to last");
    assert_eq!(ev, PickerEvent::Pending);
}

#[test]
fn color_picker_enter_commits_named_color() {
    // Index 0 = "red"
    let mut sel = 0usize;
    let ev = cp_handle(key(KeyCode::Enter), &mut sel);
    assert_eq!(ev, PickerEvent::Commit(NamedColor::Red));
}

#[test]
fn color_picker_enter_at_none_entry_commits_none() {
    // Last entry is "(none)"
    let mut sel = ENTRY_COUNT - 1;
    let ev = cp_handle(key(KeyCode::Enter), &mut sel);
    assert_eq!(ev, PickerEvent::CommitNone);
}

#[test]
fn color_picker_esc_cancels() {
    let mut sel = 3usize;
    let ev = cp_handle(key(KeyCode::Esc), &mut sel);
    assert_eq!(ev, PickerEvent::Cancel);
    assert_eq!(sel, 3, "selection unchanged on Cancel");
}

#[test]
fn color_picker_unrecognised_key_is_pending() {
    let mut sel = 2usize;
    let ev = cp_handle(key(KeyCode::Char('z')), &mut sel);
    assert_eq!(ev, PickerEvent::Pending);
    assert_eq!(sel, 2);
}

// ── save ─────────────────────────────────────────────────────────────────────

#[test]
fn save_writes_file_with_correct_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let cfg = minimal_config();
    save(&path, &cfg).expect("save must succeed");
    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    let back: Config = serde_json::from_str(&content).expect("must round-trip");
    assert_eq!(back.lines.len(), cfg.lines.len());
}

#[test]
fn save_creates_bak_only_on_first_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let bak_path = dir.path().join("config.json.bak");

    // First save: no pre-existing file, no bak expected.
    let cfg1 = minimal_config();
    save(&path, &cfg1).expect("first save");
    assert!(!bak_path.exists(), ".bak must NOT exist after first save");

    // Second save: file exists now, .bak does not → bak is created with first config.
    let mut cfg2 = minimal_config();
    cfg2.powerline = true;
    save(&path, &cfg2).expect("second save");
    assert!(bak_path.exists(), ".bak must be created on second save");
    let bak_content = std::fs::read_to_string(&bak_path).unwrap();
    let bak_cfg: Config = serde_json::from_str(&bak_content).unwrap();
    assert!(!bak_cfg.powerline, ".bak holds the first config");

    // Third save: .bak already exists → must not be overwritten.
    let mut cfg3 = minimal_config();
    cfg3.powerline = false;
    save(&path, &cfg3).expect("third save");
    let bak_after = std::fs::read_to_string(&bak_path).unwrap();
    let bak_cfg3: Config = serde_json::from_str(&bak_after).unwrap();
    assert!(!bak_cfg3.powerline, ".bak unchanged on subsequent saves");
    // Verify .bak still holds the FIRST config (powerline=false), not second (powerline=true).
    let bak_final: Config = serde_json::from_str(&bak_after).unwrap();
    assert!(
        !bak_final.powerline,
        ".bak still holds original pre-TUI snapshot"
    );
}

#[test]
fn save_atomic_via_tmp_then_rename() {
    // After save, tmp file must not remain.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let tmp_path = dir.path().join("config.json.tmp");
    save(&path, &minimal_config()).expect("save");
    assert!(!tmp_path.exists(), ".tmp must be removed after rename");
    assert!(path.exists());
}

#[test]
#[cfg(unix)]
fn save_read_only_dir_returns_error() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("ro");
    std::fs::create_dir(&sub).unwrap();

    // Make directory read-only.
    std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o555)).unwrap();

    let path = sub.join("config.json");
    let result = save(&path, &minimal_config());

    // Restore permissions so tempdir cleanup can succeed.
    std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert!(result.is_err(), "save into read-only dir must fail");
}

// ── confirm ──────────────────────────────────────────────────────────────────

#[test]
fn confirm_y_returns_true() {
    assert_eq!(confirm_handle(key(KeyCode::Char('y'))), Some(true));
    assert_eq!(confirm_handle(key(KeyCode::Char('Y'))), Some(true));
}

#[test]
fn confirm_n_returns_false() {
    assert_eq!(confirm_handle(key(KeyCode::Char('n'))), Some(false));
    assert_eq!(confirm_handle(key(KeyCode::Char('N'))), Some(false));
}

#[test]
fn confirm_esc_returns_false() {
    assert_eq!(confirm_handle(key(KeyCode::Esc)), Some(false));
}

#[test]
fn confirm_other_key_returns_none() {
    assert_eq!(confirm_handle(key(KeyCode::Char('z'))), None);
    assert_eq!(confirm_handle(key(KeyCode::Enter)), None);
    assert_eq!(confirm_handle(key(KeyCode::Tab)), None);
}

// ── help ─────────────────────────────────────────────────────────────────────

#[test]
fn help_question_mark_dismisses() {
    assert!(help_handle(key(KeyCode::Char('?'))));
}

#[test]
fn help_any_key_dismisses() {
    assert!(help_handle(key(KeyCode::Enter)));
    assert!(help_handle(key(KeyCode::Esc)));
    assert!(help_handle(key(KeyCode::Char('x'))));
}

// ── ConfirmKind formatting sanity ────────────────────────────────────────────

#[test]
fn confirm_kind_delete_line_segments_field() {
    let k = ConfirmKind::DeleteLine { segments: 3 };
    assert!(matches!(k, ConfirmKind::DeleteLine { segments: 3 }));
}

#[test]
fn confirm_kind_quit_dirty() {
    let k = ConfirmKind::QuitDirty;
    assert_eq!(k, ConfirmKind::QuitDirty);
}

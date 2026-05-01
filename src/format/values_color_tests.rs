//! Tests for ansi_fg and ansi_bg color helpers.

use super::*;

// ── ansi_fg ──────────────────────────────────────────────────────────────────

#[test]
fn ansi_fg_red_returns_fg_escape() {
    assert_eq!(ansi_fg("red"), "\x1b[31m");
}

#[test]
fn ansi_fg_green_returns_fg_escape() {
    assert_eq!(ansi_fg("green"), "\x1b[32m");
}

#[test]
fn ansi_fg_yellow_returns_fg_escape() {
    assert_eq!(ansi_fg("yellow"), "\x1b[33m");
}

#[test]
fn ansi_fg_blue_returns_fg_escape() {
    assert_eq!(ansi_fg("blue"), "\x1b[34m");
}

#[test]
fn ansi_fg_magenta_returns_fg_escape() {
    assert_eq!(ansi_fg("magenta"), "\x1b[35m");
}

#[test]
fn ansi_fg_cyan_returns_fg_escape() {
    assert_eq!(ansi_fg("cyan"), "\x1b[36m");
}

#[test]
fn ansi_fg_white_returns_fg_escape() {
    assert_eq!(ansi_fg("white"), "\x1b[37m");
}

#[test]
fn ansi_fg_default_returns_reset_fg() {
    assert_eq!(ansi_fg("default"), "\x1b[39m");
}

#[test]
fn ansi_fg_unknown_returns_empty() {
    assert_eq!(ansi_fg("purple"), "");
    assert_eq!(ansi_fg(""), "");
    assert_eq!(ansi_fg("RED"), "");
}

// ── ansi_bg ──────────────────────────────────────────────────────────────────

#[test]
fn ansi_bg_red_returns_bg_escape() {
    assert_eq!(ansi_bg("red"), "\x1b[41m");
}

#[test]
fn ansi_bg_green_returns_bg_escape() {
    assert_eq!(ansi_bg("green"), "\x1b[42m");
}

#[test]
fn ansi_bg_yellow_returns_bg_escape() {
    assert_eq!(ansi_bg("yellow"), "\x1b[43m");
}

#[test]
fn ansi_bg_blue_returns_bg_escape() {
    assert_eq!(ansi_bg("blue"), "\x1b[44m");
}

#[test]
fn ansi_bg_magenta_returns_bg_escape() {
    assert_eq!(ansi_bg("magenta"), "\x1b[45m");
}

#[test]
fn ansi_bg_cyan_returns_bg_escape() {
    assert_eq!(ansi_bg("cyan"), "\x1b[46m");
}

#[test]
fn ansi_bg_white_returns_bg_escape() {
    assert_eq!(ansi_bg("white"), "\x1b[47m");
}

#[test]
fn ansi_bg_default_returns_reset_bg() {
    assert_eq!(ansi_bg("default"), "\x1b[49m");
}

#[test]
fn ansi_bg_unknown_returns_empty() {
    assert_eq!(ansi_bg("purple"), "");
    assert_eq!(ansi_bg(""), "");
    assert_eq!(ansi_bg("RED"), "");
}

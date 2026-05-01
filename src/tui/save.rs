use std::fmt;
use std::io;
use std::path::Path;

use crate::config;
use crate::config::schema::{Config, ValidationError};

#[derive(Debug)]
pub enum SaveError {
    Validation(Vec<ValidationError>),
    BackupFailed(io::Error),
    WriteFailed(io::Error),
    SerializeFailed(serde_json::Error),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::Validation(errs) => {
                write!(f, "validation failed: {} error(s)", errs.len())
            }
            SaveError::BackupFailed(e) => write!(f, "backup failed: {e}"),
            SaveError::WriteFailed(e) => write!(f, "write failed: {e}"),
            SaveError::SerializeFailed(e) => write!(f, "serialize failed: {e}"),
        }
    }
}

/// Save `config` to `output_path` with pre-save validation and atomic write.
///
/// Steps:
/// 1. Clone config, call `validate_and_clamp`. Return `Validation` errors if any.
/// 2. If output_path exists, back it up to `<output_path>.bak`.
/// 3. Serialize via `config::print_config`.
/// 4. Write to `<output_path>.tmp`, then rename atomically.
pub fn save(config: &Config, output_path: &Path) -> Result<(), SaveError> {
    let mut cfg = config.clone();
    if let Err(errors) = cfg.validate_and_clamp() {
        return Err(SaveError::Validation(errors));
    }

    if output_path.exists() {
        let bak = bak_path(output_path);
        std::fs::copy(output_path, &bak).map_err(SaveError::BackupFailed)?;
    }

    let json = config::print_config(&cfg);
    // Serialize via print_config returns a String; a JSON error is unreachable
    // in practice, but retain the variant for forward-compatibility.
    if json == "{}" {
        // print_config falls back to "{}" only on serialization failure
        return Err(SaveError::SerializeFailed(
            serde_json::from_str::<serde_json::Value>("not json").unwrap_err(),
        ));
    }

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(SaveError::WriteFailed)?;
        }
    }

    let tmp = tmp_path(output_path);
    std::fs::write(&tmp, json.as_bytes()).map_err(SaveError::WriteFailed)?;
    std::fs::rename(&tmp, output_path).map_err(SaveError::WriteFailed)?;

    Ok(())
}

fn bak_path(p: &Path) -> std::path::PathBuf {
    let mut s = p.as_os_str().to_os_string();
    s.push(".bak");
    std::path::PathBuf::from(s)
}

fn tmp_path(p: &Path) -> std::path::PathBuf {
    let mut s = p.as_os_str().to_os_string();
    s.push(".tmp");
    std::path::PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{Line, Segment, TemplateSegment};

    fn valid_config() -> Config {
        Config {
            schema_url: None,
            powerline: false,
            lines: vec![Line {
                separator: String::new(),
                segments: vec![Segment::Template(TemplateSegment::new("{model}"))],
            }],
        }
    }

    #[test]
    fn save_writes_valid_config_to_target() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let cfg = valid_config();
        save(&cfg, &path).expect("save must succeed");
        assert!(path.exists());
        let loaded = config::from_file(&path).expect("round-trip load");
        assert_eq!(loaded.lines.len(), cfg.lines.len());
        assert_eq!(loaded.powerline, cfg.powerline);
    }

    #[test]
    fn save_creates_bak_when_target_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, b"original content").unwrap();
        let cfg = valid_config();
        save(&cfg, &path).expect("save must succeed");
        let bak = bak_path(&path);
        assert!(bak.exists(), ".bak file must be created");
        let bak_content = std::fs::read_to_string(&bak).unwrap();
        assert_eq!(bak_content, "original content");
    }

    #[test]
    fn save_skips_bak_when_target_does_not_exist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let cfg = valid_config();
        save(&cfg, &path).expect("save must succeed");
        let bak = bak_path(&path);
        assert!(!bak.exists(), ".bak must NOT appear on first save");
    }

    #[test]
    fn save_returns_validation_error_when_config_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        // Construct a Config with 4 lines to trigger TooManyLines.
        let mut cfg = valid_config();
        cfg.lines.push(cfg.lines[0].clone());
        cfg.lines.push(cfg.lines[0].clone());
        cfg.lines.push(cfg.lines[0].clone());
        assert_eq!(cfg.lines.len(), 4);
        let result = save(&cfg, &path);
        assert!(
            matches!(result, Err(SaveError::Validation(_))),
            "expected Validation error, got {:?}",
            result
        );
        assert!(
            !path.exists(),
            "target must NOT be written on validation error"
        );
    }

    #[test]
    fn save_creates_parent_dir_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("config.json");
        let cfg = valid_config();
        save(&cfg, &nested).expect("save must create parent dirs");
        assert!(nested.exists());
    }
}

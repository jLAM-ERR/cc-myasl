use std::io;
use std::path::{Path, PathBuf};

use crate::config::schema::Config;

/// Atomically write `config` to `path`.
///
/// Backup rule: if `path` exists AND `path.bak` does NOT exist, copy `path`
/// to `path.bak` before writing (preserves the user's pre-TUI snapshot; subsequent
/// saves leave `.bak` untouched).
///
/// Write sequence: serialize → tmp → fsync → rename → fsync parent (best-effort).
/// Returns the final path on success.
pub fn save(path: &Path, config: &Config) -> Result<PathBuf, io::Error> {
    let tmp_path = with_ext(path, "tmp");
    let bak_path = with_ext(path, "bak");

    if path.exists() && !bak_path.exists() {
        std::fs::copy(path, &bak_path)?;
    }

    let json = serde_json::to_string_pretty(config)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    {
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(json.as_bytes())?;
        file.flush()?;
        file.sync_all()?;
    }

    std::fs::rename(&tmp_path, path)?;

    // fsync parent dir — best-effort; ignore errors.
    if let Some(parent) = path.parent() {
        if let Ok(dir) = std::fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }

    Ok(path.to_path_buf())
}

fn with_ext(p: &Path, ext: &str) -> PathBuf {
    let mut s = p.as_os_str().to_os_string();
    s.push(".");
    s.push(ext);
    PathBuf::from(s)
}

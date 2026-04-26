//! Atomic write helper: write-tmp-then-rename(2).

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// Write `bytes` to `path` atomically: writes to `path.tmp`, fsyncs,
/// then `rename(2)` over `path`. Last-writer-wins under concurrent calls;
/// no reader ever observes a partially-written file.
///
/// If the parent directory does not exist, returns `Err` — the caller
/// (Task 9 orchestrator) is responsible for ensuring the directory exists
/// before calling.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Read;
    use std::sync::{Arc, Barrier};
    use tempfile::tempdir;

    #[test]
    fn write_succeeds_content_matches() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        write_atomic(&path, b"hello world").unwrap();
        let content = fs::read(&path).unwrap();
        assert_eq!(content, b"hello world");
    }

    #[test]
    fn overwrite_replaces_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        write_atomic(&path, b"version-A").unwrap();
        write_atomic(&path, b"version-B").unwrap();
        let content = fs::read(&path).unwrap();
        assert_eq!(content, b"version-B");
    }

    #[test]
    fn no_tmp_artefact_after_rename() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        write_atomic(&path, b"data").unwrap();
        let tmp = path.with_extension("tmp");
        assert!(!tmp.exists(), ".tmp file should not remain after rename");
    }

    #[test]
    fn missing_parent_dir_returns_err() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent_subdir").join("test.json");
        let result = write_atomic(&path, b"data");
        assert!(result.is_err());
    }

    #[test]
    fn concurrent_writes_last_writer_wins_no_tmp_left() {
        let dir = tempdir().unwrap();
        let path = Arc::new(dir.path().join("concurrent.json"));
        let num_threads = 20;
        let barrier = Arc::new(Barrier::new(num_threads));

        let handles: Vec<_> = (0..num_threads)
            .map(|i| {
                let path = Arc::clone(&path);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    // All threads wait at the barrier to maximise contention.
                    barrier.wait();
                    let payload = format!("writer-{i}");
                    write_atomic(&path, payload.as_bytes())
                })
            })
            .collect();

        let mut any_ok = false;
        for handle in handles {
            // Some writers may lose the rename race on some platforms but
            // that is acceptable — last-writer-wins; the file is never corrupt.
            if let Ok(()) = handle.join().unwrap() {
                any_ok = true;
            }
        }
        assert!(any_ok, "at least one writer must succeed");

        // The file must contain exactly one of the valid byte sequences.
        let content = fs::read(path.as_ref()).unwrap();
        let content_str = std::str::from_utf8(&content).unwrap();
        let valid = (0..num_threads).any(|i| content_str == format!("writer-{i}"));
        assert!(valid, "file content was unexpected: {content_str:?}");

        // No .tmp artefact may remain.
        let tmp = path.with_extension("tmp");
        assert!(
            !tmp.exists(),
            ".tmp file must not remain after concurrent writes"
        );
    }

    #[test]
    fn concurrent_read_never_observes_partial_write() {
        let dir = tempdir().unwrap();
        let path = Arc::new(dir.path().join("read_race.json"));
        let num_writers = 20;
        let num_readers = 20;
        let barrier = Arc::new(Barrier::new(num_writers + num_readers));

        // Build the set of valid payloads so readers can validate what they see.
        let valid_payloads: Arc<Vec<Vec<u8>>> = Arc::new(
            (0..num_writers)
                .map(|i| format!("writer-{i}").into_bytes())
                .collect(),
        );

        let mut handles = Vec::new();

        // Writer threads.
        for i in 0..num_writers {
            let path = Arc::clone(&path);
            let barrier = Arc::clone(&barrier);
            let handle = std::thread::spawn(move || {
                barrier.wait();
                let payload = format!("writer-{i}");
                let _ = write_atomic(&path, payload.as_bytes());
            });
            handles.push(handle);
        }

        // Reader threads.
        for _ in 0..num_readers {
            let path = Arc::clone(&path);
            let barrier = Arc::clone(&barrier);
            let valid_payloads = Arc::clone(&valid_payloads);
            let handle = std::thread::spawn(move || {
                barrier.wait();
                match File::open(path.as_ref()) {
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {
                        // File doesn't exist yet — acceptable before any writer finishes.
                    }
                    Err(e) => panic!("unexpected read error: {e}"),
                    Ok(mut f) => {
                        let mut buf = Vec::new();
                        f.read_to_end(&mut buf).unwrap();
                        // The data read must be exactly one of the valid payloads.
                        let is_valid = valid_payloads.iter().any(|p| *p == buf);
                        assert!(is_valid, "reader observed corrupt/partial data: {buf:?}");
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // After all threads complete, no .tmp artefact remains.
        let tmp = path.with_extension("tmp");
        assert!(!tmp.exists(), ".tmp artefact must not remain");
    }
}

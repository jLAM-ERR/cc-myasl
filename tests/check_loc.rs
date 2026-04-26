use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Create a temporary src/ directory with a .rs file of exactly `n` lines.
fn make_fixture(n: usize) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    let file = src.join("fixture.rs");
    // Each line is "// line N\n"
    let content: String = (1..=n).fold(String::new(), |mut acc, i| {
        use std::fmt::Write;
        let _ = writeln!(acc, "// line {i}");
        acc
    });
    fs::write(&file, content).unwrap();
    dir
}

fn run_check_loc(working_dir: &std::path::Path) -> std::process::Output {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("check-loc.sh");
    Command::new("sh")
        .arg(&script)
        .current_dir(working_dir)
        .output()
        .expect("failed to run check-loc.sh")
}

#[test]
fn check_loc_passes_at_499_lines() {
    let dir = make_fixture(499);
    let out = run_check_loc(dir.path());
    assert!(
        out.status.success(),
        "expected exit 0 for 499-line file, got:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn check_loc_passes_at_500_lines() {
    let dir = make_fixture(500);
    let out = run_check_loc(dir.path());
    assert!(
        out.status.success(),
        "expected exit 0 for 500-line file, got:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn check_loc_fails_at_501_lines() {
    let dir = make_fixture(501);
    let out = run_check_loc(dir.path());
    assert!(
        !out.status.success(),
        "expected exit 1 for 501-line file, but got exit 0"
    );
}

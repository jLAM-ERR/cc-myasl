//! Integration test: `--configure` exits 1 with an error message when
//! stdin or stdout is not a TTY.

use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("cc-myasl").expect("binary must build")
}

/// Spawn `--configure --output <rand>` with stdin piped (empty) and stdout
/// captured. Both make the process non-TTY, so exit code must be 1 and
/// stderr must contain the expected message.
#[test]
fn configure_flag_with_no_tty_exits_one_with_message() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out_path = tmp.path().join("cfg_out.json");

    // write_stdin("") causes assert_cmd to pipe stdin (not a TTY).
    // assert_cmd also captures stdout (pipe, not a TTY).
    bin()
        .args(["--configure", "--output", out_path.to_str().unwrap()])
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicates::str::contains("interactive terminal"));
}

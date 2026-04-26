use assert_cmd::Command;
use predicates::prelude::*;

/// Spawn the binary with empty stdin, assert non-empty stdout and exit code 0.
#[test]
fn smoke_empty_stdin_exits_zero_with_output() {
    let mut cmd = Command::cargo_bin("cc-myasl").unwrap();
    cmd.write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

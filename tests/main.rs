use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_zero_installed_tf_versions() {
    let mut cmd = Command::cargo_bin("terve").unwrap();
    let assert = cmd.arg("l").arg("tf").assert();
    assert.success().code(0).stdout(predicate::eq(""));
}

#[test]
fn test_zero_installed_tg_versions() {
    let mut cmd = Command::cargo_bin("terve").unwrap();
    let assert = cmd.arg("l").arg("tg").assert();
    assert.success().code(0).stdout(predicate::eq(""));
}

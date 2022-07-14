use assert_cmd::prelude::*;
use dirs::home_dir;
use predicates::{
    prelude::*,
    str::{contains, is_empty, starts_with},
};
use same_file::is_same_file;
use std::{env::consts::EXE_SUFFIX, path::PathBuf, process::Command};
use tempfile::tempdir;

#[test]
fn test_terraform_workflow() {
    let home_dir = get_home_dir();

    // List installed versions (none initially)
    terve(&home_dir)
        .arg("l")
        .arg("tf")
        .assert()
        .success()
        .code(0)
        .stdout(is_empty());

    // List remote versions
    terve(&home_dir)
        .arg("l")
        .arg("tf")
        .arg("r")
        .assert()
        .success()
        .code(0)
        .stdout(contains("1.2.4").and(contains("1.1.3")));

    // Try install non-existent version
    terve(&home_dir)
        .arg("i")
        .arg("tf")
        .arg("1.0.0-nope.1")
        .assert()
        .failure()
        .code(1)
        .stderr(starts_with("ERROR: HTTP status client error"));

    // Install some version
    terve(&home_dir)
        .arg("i")
        .arg("tf")
        .arg("1.2.4")
        .assert()
        .success()
        .code(0)
        .stdout(contains("Installed terraform 1.2.4"));

    // Assert idempotency by running install twice for another version
    for _ in 1..=2 {
        terve(&home_dir)
            .arg("i")
            .arg("tf")
            .arg("1.1.3")
            .assert()
            .success()
            .code(0)
            .stdout(contains("Installed terraform 1.1.3"));
    }

    // Try to select a non-installed version
    terve(&home_dir)
        .arg("s")
        .arg("tf")
        .arg("0.14.10")
        .assert()
        .failure()
        .code(1)
        .stderr(contains(
            "ERROR: terraform version 0.14.10 is not installed",
        ));

    // Assert idempotency by running select twice
    for _ in 1..=2 {
        terve(&home_dir)
            .arg("s")
            .arg("tf")
            .arg("1.1.3")
            .assert()
            .success()
            .code(0)
            .stdout(contains("Selected terraform 1.1.3"));
    }

    // List installed versions (expecting two)
    terve(&home_dir)
        .arg("l")
        .arg("tf")
        .assert()
        .success()
        .code(0)
        .stdout(contains("1.2.4").and(contains("1.1.3")));

    // Show selected version
    terve(&home_dir)
        .arg("w")
        .arg("tf")
        .assert()
        .success()
        .code(0)
        .stdout(contains("1.1.3"));

    // Assert hard link points to expected terraform binary

    let hard_link_path = home_dir
        .join(".terve")
        .join("bin")
        .join(format!("terraform{}", EXE_SUFFIX));

    let opt_file_path = home_dir
        .join(".terve")
        .join("opt")
        .join(format!("terraform{}", EXE_SUFFIX))
        .join("1.1.3");

    assert!(is_same_file(&hard_link_path, &opt_file_path).unwrap());

    Command::new(hard_link_path)
        .arg("--version")
        .assert()
        .success()
        .code(0)
        .stdout(contains("1.1.3"));

    // Assert idempotency by running remove twice
    for _ in 1..=2 {
        terve(&home_dir)
            .arg("r")
            .arg("tf")
            .arg("1.1.3")
            .assert()
            .success()
            .code(0)
            .stdout(contains("Removed terraform 1.1.3"));
    }

    // Assert same version is still selected
    terve(&home_dir)
        .arg("w")
        .arg("tf")
        .assert()
        .success()
        .code(0)
        .stdout(contains("1.1.3"));

    // Remove the other version
    terve(&home_dir)
        .arg("r")
        .arg("tf")
        .arg("1.2.4")
        .assert()
        .success()
        .code(0)
        .stdout(contains("Removed terraform 1.2.4"));

    // Assert no installed versions
    terve(&home_dir)
        .arg("l")
        .arg("tf")
        .assert()
        .success()
        .code(0)
        .stdout(is_empty());
}

#[test]
fn test_terragrunt_workflow() {
    let home_dir = get_home_dir();

    // List installed versions (none initially)
    terve(&home_dir)
        .arg("l")
        .arg("tg")
        .assert()
        .success()
        .code(0)
        .stdout(is_empty());

    // List remote versions
    terve(&home_dir)
        .arg("l")
        .arg("tg")
        .arg("r")
        .assert()
        .success()
        .code(0)
        .stdout(contains("0.38.4").and(contains("0.28.24")));

    // Try install non-existent version
    terve(&home_dir)
        .arg("i")
        .arg("tg")
        .arg("0.666.6-nope.1")
        .assert()
        .failure()
        .code(1)
        .stderr(starts_with("ERROR: HTTP status client error"));

    // Assert idempotency by running install twice
    for _ in 1..=2 {
        terve(&home_dir)
            .arg("i")
            .arg("tg")
            .arg("0.38.4")
            .assert()
            .success()
            .code(0)
            .stdout(contains("Installed terragrunt 0.38.4"));
    }

    // Install another version
    terve(&home_dir)
        .arg("i")
        .arg("tg")
        .arg("0.28.24")
        .assert()
        .success()
        .code(0)
        .stdout(contains("Installed terragrunt 0.28.24"));

    // Try select non-installed version
    terve(&home_dir)
        .arg("s")
        .arg("tg")
        .arg("0.28.2")
        .assert()
        .failure()
        .code(1)
        .stderr(contains(
            "ERROR: terragrunt version 0.28.2 is not installed",
        ));

    // Assert idempotency by running select twice
    for _ in 1..=2 {
        terve(&home_dir)
            .arg("s")
            .arg("tg")
            .arg("0.38.4")
            .assert()
            .success()
            .code(0)
            .stdout(contains("Selected terragrunt 0.38.4"));
    }

    if cfg!(linux) {
        // Install version for which sha256 checksum is not available
        terve(&home_dir)
            .arg("i")
            .arg("tg")
            .arg("0.18.0")
            .assert()
            .success()
            .code(0)
            .stderr(contains("WARNING: Skipping SHA256 file integrity check"))
            .stdout(contains("Installed terragrunt 0.18.0"));

        // Test for https://github.com/superblk/terve/issues/21
        let old_terragrunt_version = home_dir
            .join(".terve")
            .join("opt")
            .join(format!("terragrunt{}", EXE_SUFFIX))
            .join("0.18.0");

        Command::new(&old_terragrunt_version)
            .arg("--version")
            .assert()
            .success()
            .code(0)
            .stdout(contains("0.18.0"));

        // Remove this version
        terve(&home_dir)
            .arg("r")
            .arg("tg")
            .arg("0.18.0")
            .assert()
            .success()
            .code(0)
            .stdout(contains("Removed terragrunt 0.18.0"));
    }

    // Assert both installed versions are listed
    terve(&home_dir)
        .arg("l")
        .arg("tg")
        .assert()
        .success()
        .code(0)
        .stdout(contains("0.38.4").and(contains("0.28.24")));

    // Assert correct version is selected
    terve(&home_dir)
        .arg("w")
        .arg("tg")
        .assert()
        .success()
        .code(0)
        .stdout(contains("0.38.4"));

    let hard_link_path = home_dir
        .join(".terve")
        .join("bin")
        .join(format!("terragrunt{}", EXE_SUFFIX));

    let opt_file_path = home_dir
        .join(".terve")
        .join("opt")
        .join(format!("terragrunt{}", EXE_SUFFIX))
        .join("0.38.4");

    assert!(is_same_file(&hard_link_path, &opt_file_path).unwrap());

    Command::new(hard_link_path)
        .arg("--version")
        .assert()
        .success()
        .code(0)
        .stdout(contains("terragrunt version v0.38.4"));

    // Assert idempotency by running remove twice
    for _ in 1..=2 {
        terve(&home_dir)
            .arg("r")
            .arg("tg")
            .arg("0.38.4")
            .assert()
            .success()
            .code(0)
            .stdout(contains("Removed terragrunt 0.38.4"));
    }

    // Assert same version is still selected
    terve(&home_dir)
        .arg("w")
        .arg("tg")
        .assert()
        .success()
        .code(0)
        .stdout(contains("0.38.4"));

    // Remove the other version
    terve(&home_dir)
        .arg("r")
        .arg("tg")
        .arg("0.28.24")
        .assert()
        .success()
        .code(0)
        .stdout(contains("Removed terragrunt 0.28.24"));

    // Assert no installed versions
    terve(&home_dir)
        .arg("l")
        .arg("tg")
        .assert()
        .success()
        .code(0)
        .stdout(is_empty());
}

fn terve(home_dir: &PathBuf) -> Command {
    let mut cmd = Command::cargo_bin("terve").unwrap();
    cmd.env("HOME", &home_dir.as_os_str());
    cmd
}

fn get_home_dir() -> PathBuf {
    if cfg!(unix) {
        tempdir()
            .expect("failed to create fake home dir")
            .into_path()
    } else {
        home_dir().unwrap()
    }
}

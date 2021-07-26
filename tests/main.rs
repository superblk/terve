use assert_cmd::prelude::*;
use dirs::home_dir;
use predicates::prelude::*;
use same_file::is_same_file;
use std::{env, path::PathBuf, process::Command};
use tempfile::tempdir;

#[test]
fn test_workflows() {
    let home_dir = if cfg!(unix) {
        fake_home_dir()
    } else {
        home_dir().unwrap()
    };
    test_terraform_all(&home_dir);
    test_terragrunt_all(&home_dir);
    test_install_old_terragrunt_with_no_shasums(&home_dir);
}

fn test_terraform_all(home_dir: &PathBuf) {
    terve()
        .arg("l")
        .arg("tf")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::is_empty());

    terve()
        .arg("l")
        .arg("tf")
        .arg("r")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("0.14.11"));

    // Assert idempotency by running the command twice
    for _ in 1..=2 {
        terve()
            .arg("i")
            .arg("tf")
            .arg("0.14.11")
            .assert()
            .success()
            .code(0)
            .stdout(predicate::str::contains("Installed terraform 0.14.11"));
    }

    terve()
        .arg("s")
        .arg("tf")
        .arg("0.14.10")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("ERROR: terraform version 0.14.10 is not installed. Run 'terve install terraform 0.14.10'"));

    // Assert idempotency by running the command twice
    for _ in 1..=2 {
        terve()
            .arg("s")
            .arg("tf")
            .arg("0.14.11")
            .assert()
            .success()
            .code(0)
            .stdout(predicate::str::contains("Selected terraform 0.14.11"));
    }

    let hard_link_path = if cfg!(unix) {
        home_dir.join(".terve").join("bin").join("terraform")
    } else {
        home_dir.join(".terve").join("bin").join("terraform.exe")
    };

    let opt_file_path = if cfg!(unix) {
        home_dir
            .join(".terve")
            .join("opt")
            .join("terraform")
            .join("0.14.11")
    } else {
        home_dir
            .join(".terve")
            .join("opt")
            .join("terraform.exe")
            .join("0.14.11")
    };

    assert!(is_same_file(&hard_link_path, &opt_file_path).unwrap());

    Command::new(hard_link_path)
        .arg("--version")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("0.14.11"));

    // Assert idempotency by running the command twice
    for _ in 1..=2 {
        terve()
            .arg("r")
            .arg("tf")
            .arg("0.14.11")
            .assert()
            .success()
            .code(0)
            .stdout(predicate::str::contains("Removed terraform 0.14.11"));
    }

    terve()
        .arg("l")
        .arg("tf")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::is_empty());
}

fn test_terragrunt_all(home_dir: &PathBuf) {
    terve()
        .arg("l")
        .arg("tg")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::is_empty());

    terve()
        .arg("l")
        .arg("tg")
        .arg("r")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("0.29.2"));

    // Assert idempotency by running the command twice
    for _ in 1..=2 {
        terve()
            .arg("i")
            .arg("tg")
            .arg("0.29.2")
            .assert()
            .success()
            .code(0)
            .stdout(predicate::str::contains("Installed terragrunt 0.29.2"));
    }

    terve()
        .arg("s")
        .arg("tg")
        .arg("0.28.2")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("ERROR: terragrunt version 0.28.2 is not installed. Run 'terve install terragrunt 0.28.2'"));

    // Assert idempotency by running the command twice
    for _ in 1..=2 {
        terve()
            .arg("s")
            .arg("tg")
            .arg("0.29.2")
            .assert()
            .success()
            .code(0)
            .stdout(predicate::str::contains("Selected terragrunt 0.29.2"));
    }

    let hard_link_path = if cfg!(unix) {
        home_dir.join(".terve").join("bin").join("terragrunt")
    } else {
        home_dir.join(".terve").join("bin").join("terragrunt.exe")
    };

    let opt_file_path = if cfg!(unix) {
        home_dir
            .join(".terve")
            .join("opt")
            .join("terragrunt")
            .join("0.29.2")
    } else {
        home_dir
            .join(".terve")
            .join("opt")
            .join("terragrunt.exe")
            .join("0.29.2")
    };

    assert!(is_same_file(&hard_link_path, &opt_file_path).unwrap());

    Command::new(hard_link_path)
        .arg("--version")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("terragrunt version v0.29.2"));

    // Assert idempotency by running the command twice
    for _ in 1..=2 {
        terve()
            .arg("r")
            .arg("tg")
            .arg("0.29.2")
            .assert()
            .success()
            .code(0)
            .stdout(predicate::str::contains("Removed terragrunt 0.29.2"));
    }

    terve()
        .arg("l")
        .arg("tg")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::is_empty());
}

// See https://github.com/superblk/terve/issues/21
fn test_install_old_terragrunt_with_no_shasums(home_dir: &PathBuf) {
    terve()
        .arg("i")
        .arg("tg")
        .arg("0.18.0")
        .assert()
        .success()
        .code(0)
        .stderr(predicate::str::contains(
            "WARNING: Skipping SHA256 file integrity check",
        ))
        .stdout(predicate::str::contains("Installed terragrunt 0.18.0"));

    let opt_file_path = if cfg!(unix) {
        home_dir
            .join(".terve")
            .join("opt")
            .join("terragrunt")
            .join("0.18.0")
    } else {
        home_dir
            .join(".terve")
            .join("opt")
            .join("terragrunt.exe")
            .join("0.18.0")
    };

    assert!(opt_file_path.metadata().unwrap().len() > 0);

    Command::new(opt_file_path)
        .arg("--version")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("0.18.0"));
}

fn terve() -> Command {
    Command::cargo_bin("terve").unwrap()
}

fn fake_home_dir() -> PathBuf {
    let fake_home_dir = tempdir()
        .expect("failed to create fake home dir")
        .into_path();
    env::set_var("HOME", &fake_home_dir.as_os_str());
    fake_home_dir
}

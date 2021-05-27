use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::{env, fs::read_link, path::PathBuf, process::Command};
use tempfile::tempdir;

#[test]
fn test_workflows() {
    let fake_home = use_fake_home_dir();
    test_terraform_all(&fake_home);
    test_terragrunt_all(&fake_home);
}

fn test_terraform_all(home: &PathBuf) {
    terve()
        .arg("l")
        .arg("tf")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::eq(""));

    terve()
        .arg("l")
        .arg("tf")
        .arg("r")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("0.14.11\n"));

    // Assert idempotency by running the command twice
    for _ in 1..=2 {
        terve()
            .arg("i")
            .arg("tf")
            .arg("0.14.11")
            .assert()
            .success()
            .code(0)
            .stdout(predicate::eq("Installed terraform 0.14.11\n"));
    }

    terve()
        .arg("s")
        .arg("tf")
        .arg("0.14.10")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::eq("ERROR: terraform version 0.14.10 is not installed. Run 'terve install terraform 0.14.10'\n"));

    // Assert idempotency by running the command twice
    for _ in 1..=2 {
        terve()
            .arg("s")
            .arg("tf")
            .arg("0.14.11")
            .assert()
            .success()
            .code(0)
            .stdout(predicate::eq("Selected terraform 0.14.11\n"));
    }

    let symlink_path = home.join(".terve/bin/terraform");
    let opt_file_path = home.join(".terve/opt/terraform/0.14.11");

    assert!(
        symlink_path.exists()
            && opt_file_path.exists()
            && read_link(symlink_path).expect("Failed to read symlink") == opt_file_path
    );

    // Assert idempotency by running the command twice
    for _ in 1..=2 {
        terve()
            .arg("r")
            .arg("tf")
            .arg("0.14.11")
            .assert()
            .success()
            .code(0)
            .stdout(predicate::eq("Removed terraform 0.14.11\n"));
    }

    terve()
        .arg("l")
        .arg("tf")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::eq(""));
}

fn test_terragrunt_all(home: &PathBuf) {
    terve()
        .arg("l")
        .arg("tg")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::eq(""));

    terve()
        .arg("l")
        .arg("tg")
        .arg("r")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("0.29.2\n"));

    // Assert idempotency by running the command twice
    for _ in 1..=2 {
        terve()
            .arg("i")
            .arg("tg")
            .arg("0.29.2")
            .assert()
            .success()
            .code(0)
            .stdout(predicate::eq("Installed terragrunt 0.29.2\n"));
    }

    terve()
        .arg("s")
        .arg("tg")
        .arg("0.28.2")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::eq("ERROR: terragrunt version 0.28.2 is not installed. Run 'terve install terragrunt 0.28.2'\n"));

    // Assert idempotency by running the command twice
    for _ in 1..=2 {
        terve()
            .arg("s")
            .arg("tg")
            .arg("0.29.2")
            .assert()
            .success()
            .code(0)
            .stdout(predicate::eq("Selected terragrunt 0.29.2\n"));
    }

    let symlink_path = home.join(".terve/bin/terragrunt");
    let opt_file_path = home.join(".terve/opt/terragrunt/0.29.2");

    assert!(
        symlink_path.exists()
            && opt_file_path.exists()
            && read_link(symlink_path).expect("Failed to read symlink") == opt_file_path
    );

    // Assert idempotency by running the command twice
    for _ in 1..=2 {
        terve()
            .arg("r")
            .arg("tg")
            .arg("0.29.2")
            .assert()
            .success()
            .code(0)
            .stdout(predicate::eq("Removed terragrunt 0.29.2\n"));
    }

    terve()
        .arg("l")
        .arg("tg")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::eq(""));
}

fn terve() -> Command {
    Command::cargo_bin("terve").unwrap()
}

fn use_fake_home_dir() -> PathBuf {
    let fake_home = tempdir()
        .expect("failed to create fake home dir")
        .into_path();
    env::set_var("HOME", &fake_home.as_os_str());
    fake_home
}

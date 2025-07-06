use assert_cmd::Command;
use predicates::prelude::*;
use packaging::utils::{artifact_path, workspace_version};
use serial_test::serial;
use std::fs;

#[test]
#[serial]
fn test_ci_checks_binary() -> Result<(), Box<dyn std::error::Error>> {
    let version = workspace_version()?;
    let path = artifact_path(&version);

    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, b"test")?;

    Command::cargo_bin("ci_checks")?
        .assert()
        .success()
        .stdout(predicate::str::contains("CI checks passed"));

    fs::remove_file(path)?;
    Ok(())
}

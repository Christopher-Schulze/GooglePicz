use assert_cmd::Command;
use predicates::prelude::*;
use packaging::utils::{get_project_root, workspace_version};
use serial_test::serial;
use std::fs;

#[test]
#[serial]
fn test_ci_checks_binary() -> Result<(), Box<dyn std::error::Error>> {
    let version = workspace_version()?;
    let root = get_project_root();

    #[cfg(target_os = "linux")]
    let path = root.join(format!("GooglePicz-{}.deb", version));
    #[cfg(target_os = "macos")]
    let path = root.join(format!("target/release/GooglePicz-{}.dmg", version));
    #[cfg(target_os = "windows")]
    let path = root.join(format!("target/windows/GooglePicz-{}-Setup.exe", version));

    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, b"test")?;

    Command::cargo_bin("ci_checks")?
        .assert()
        .success()
        .stdout(predicate::str::contains("CI checks passed"));

    fs::remove_file(path)?;
    Ok(())
}

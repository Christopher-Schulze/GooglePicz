use assert_cmd::prelude::*;
use predicates::str::contains;
use std::process::Command;
use tempfile::TempDir;

fn build_cmd(home: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("sync_cli").unwrap();
    cmd.env("MOCK_API_CLIENT", "1");
    cmd.env("MOCK_KEYRING", "1");
    cmd.env("MOCK_ACCESS_TOKEN", "token");
    cmd.env("MOCK_REFRESH_TOKEN", "refresh");
    cmd.env("HOME", home);
    cmd
}

#[test]
fn sync_command_runs() {
    let dir = TempDir::new().unwrap();
    build_cmd(dir.path())
        .arg("sync")
        .assert()
        .success()
        .stdout(contains("Finished sync"));
}

#[test]
fn status_after_sync_shows_info() {
    let dir = TempDir::new().unwrap();
    build_cmd(dir.path())
        .arg("sync")
        .assert()
        .success();

    build_cmd(dir.path())
        .arg("status")
        .assert()
        .success()
        .stdout(contains("Last sync"))
        .stdout(contains("Cached items"));
}

#[test]
fn list_albums_after_create() {
    let dir = TempDir::new().unwrap();
    build_cmd(dir.path())
        .arg("sync")
        .assert()
        .success();

    build_cmd(dir.path())
        .args(&["create-album", "Test"])
        .assert()
        .success()
        .stdout(contains("Album created"));

    build_cmd(dir.path())
        .arg("list-albums")
        .assert()
        .success()
        .stdout(contains("Test"))
        .stdout(contains("(id: 1)"));
}

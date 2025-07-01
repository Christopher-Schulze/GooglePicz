use assert_cmd::prelude::*;
use std::process::Command;
use tempfile::tempdir;

// Helper function to create command with mocked environment
fn cli_command() -> Command {
    let dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("sync_cli").unwrap();
    cmd.env("MOCK_API_CLIENT", "1");
    cmd.env("MOCK_KEYRING", "1");
    cmd.env("HOME", dir.path());
    cmd
}

#[test]
fn test_help() {
    cli_command()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("GooglePicz"));
}

#[test]
fn test_status() {
    cli_command()
        .arg("status")
        .assert()
        .success();
}

#[test]
fn test_list_albums() {
    cli_command()
        .arg("list-albums")
        .assert()
        .success();
}

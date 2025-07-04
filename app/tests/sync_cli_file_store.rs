use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn sync_cli_creates_token_file() {
    let dir = TempDir::new().unwrap();
    let token_path = dir.path().join(".googlepicz").join("tokens.json");
    let mut cmd = Command::cargo_bin("sync_cli").unwrap();
    cmd.arg("--use-file-store")
        .arg("sync")
        .env("MOCK_API_CLIENT", "1")
        .env("MOCK_ACCESS_TOKEN", "tok")
        .env("MOCK_REFRESH_TOKEN", "ref")
        .env("HOME", dir.path());
    cmd.assert().success().stdout(predicate::str::contains("Finished sync"));
    assert!(token_path.exists());
}

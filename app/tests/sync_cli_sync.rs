use assert_cmd::prelude::*;
use predicates::str::contains;
use tempfile::TempDir;
use std::process::Command;

#[test]
fn sync_cli_sync_mock() {
    let tmp_home = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("sync_cli").unwrap();
    cmd.arg("sync");
    cmd.env("MOCK_API_CLIENT", "1");
    cmd.env("MOCK_KEYRING", "1");
    cmd.env("MOCK_ACCESS_TOKEN", "token");
    cmd.env("MOCK_REFRESH_TOKEN", "refresh");
    cmd.env("HOME", tmp_home.path());
    cmd.assert()
        .success()
        .stdout(contains("Finished sync"));
}

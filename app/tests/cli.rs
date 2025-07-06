use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn sync_cli_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("sync_cli")?;
    cmd.arg("--help");
    cmd.env("MOCK_API_CLIENT", "1");
    cmd.env("MOCK_KEYRING", "1");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("GooglePicz synchronization CLI"));
    Ok(())
}

#[test]
fn sync_cli_status_no_cache() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_home = TempDir::new()?;
    let mut cmd = Command::cargo_bin("sync_cli")?;
    cmd.arg("status");
    cmd.env("MOCK_API_CLIENT", "1");
    cmd.env("MOCK_KEYRING", "1");
    cmd.env("HOME", tmp_home.path());
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No cache found"));
    Ok(())
}

#[test]
fn sync_cli_list_albums_no_cache() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_home = TempDir::new()?;
    let mut cmd = Command::cargo_bin("sync_cli")?;
    cmd.arg("list-albums");
    cmd.env("MOCK_API_CLIENT", "1");
    cmd.env("MOCK_KEYRING", "1");
    cmd.env("HOME", tmp_home.path());
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No cache found"));
    Ok(())
}

#[test]
fn sync_cli_cache_stats_no_cache() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_home = TempDir::new()?;
    let mut cmd = Command::cargo_bin("sync_cli")?;
    cmd.arg("cache-stats");
    cmd.env("MOCK_API_CLIENT", "1");
    cmd.env("MOCK_KEYRING", "1");
    cmd.env("HOME", tmp_home.path());
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No cache found"));
    Ok(())
}

#[test]
fn sync_cli_search_no_cache() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_home = TempDir::new()?;
    let mut cmd = Command::cargo_bin("sync_cli")?;
    cmd.args(["search", "test"]);
    cmd.env("MOCK_API_CLIENT", "1");
    cmd.env("MOCK_KEYRING", "1");
    cmd.env("HOME", tmp_home.path());
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No cache found"));
    Ok(())
}


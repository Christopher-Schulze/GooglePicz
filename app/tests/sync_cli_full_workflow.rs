use assert_cmd::prelude::*;
use predicates::prelude::*;
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

fn synced_home() -> TempDir {
    let dir = TempDir::new().unwrap();
    build_cmd(dir.path())
        .arg("sync")
        .assert()
        .success()
        .stdout(predicate::str::contains("Finished sync"));
    dir
}

#[test]
fn cache_stats_after_full_sync() {
    let dir = synced_home();
    build_cmd(dir.path())
        .arg("cache-stats")
        .assert()
        .success()
        .stdout(predicate::str::contains("Albums: 0"))
        .stdout(predicate::str::contains("Media items: 1"));
}

#[test]
fn delete_album_after_full_sync() {
    let dir = synced_home();
    build_cmd(dir.path())
        .args(&["create-album", "Temp"])
        .assert()
        .success();

    build_cmd(dir.path())
        .args(&["delete-album", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Album deleted"));

    build_cmd(dir.path())
        .arg("list-albums")
        .assert()
        .success()
        .stdout(predicate::str::contains("Temp").not());
}

#[test]
fn clear_cache_after_full_sync() {
    let dir = synced_home();
    build_cmd(dir.path())
        .arg("clear-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache cleared"));

    build_cmd(dir.path())
        .arg("cache-stats")
        .assert()
        .success()
        .stdout(predicate::str::contains("Albums: 0"))
        .stdout(predicate::str::contains("Media items: 0"));

    build_cmd(dir.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("1970-01-01"))
        .stdout(predicate::str::contains("Cached items: 0"));
}

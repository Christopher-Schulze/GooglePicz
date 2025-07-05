use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

fn build_cmd() -> Command {
    let dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("sync_cli").unwrap();
    cmd.env("MOCK_API_CLIENT", "1");
    cmd.env("MOCK_KEYRING", "1");
    cmd.env("HOME", dir.path());
    cmd
}

#[test]
fn sync_cli_help_runs() {
    build_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("GooglePicz synchronization CLI"));
}

#[test]
fn sync_cli_status_no_cache() {
    build_cmd()
        .arg("status")
        .assert()
        .success()
        .stdout(contains("No cache found"));
}

#[test]
fn sync_cli_list_albums_no_cache() {
    build_cmd()
        .arg("list-albums")
        .assert()
        .success()
        .stdout(contains("No cache found"));
}

#[test]
fn sync_cli_create_album_no_cache() {
    build_cmd()
        .args(&["create-album", "Test"])
        .assert()
        .success()
        .stdout(contains("No cache found"));
}

#[test]
fn sync_cli_delete_album_no_cache() {
    build_cmd()
        .args(&["delete-album", "1"])
        .assert()
        .success()
        .stdout(contains("No cache found"));
}

#[test]
fn sync_cli_cache_stats_no_cache() {
    build_cmd()
        .arg("cache-stats")
        .assert()
        .success()
        .stdout(contains("No cache found"));
}

#[test]
fn sync_cli_rename_album_no_cache() {
    build_cmd()
        .args(&["rename-album", "1", "NewTitle"])
        .assert()
        .success()
        .stdout(contains("No cache found"));
}

#[test]
fn sync_cli_add_to_album_no_cache() {
    build_cmd()
        .args(&["add-to-album", "1", "2"])
        .assert()
        .success()
        .stdout(contains("No cache found"));
}

#[test]
fn sync_cli_list_album_items_no_cache() {
    build_cmd()
        .args(&["list-album-items", "1"])
        .assert()
        .success()
        .stdout(contains("No cache found"));
}

#[test]
fn sync_cli_export_albums_no_cache() {
    build_cmd()
        .args(&["export-albums", "--file", "out.json"])
        .assert()
        .success()
        .stdout(contains("No cache found"));
}

use packaging::{create_installer};
use packaging::utils::{get_project_root, workspace_version, verify_artifact_names};
use serial_test::serial;
use std::fs;

#[cfg(target_os = "linux")]
#[test]
#[serial]
fn test_create_installer_linux_mock() {
    std::env::set_var("MOCK_COMMANDS", "1");
    std::env::set_var("LINUX_PACKAGE_FORMAT", "deb");
    let root = get_project_root();
    let deb_dir = root.join("target/debian");
    fs::create_dir_all(&deb_dir).unwrap();
    fs::write(deb_dir.join("dummy.deb"), b"test").unwrap();

    let result = create_installer();
    assert!(result.is_ok());
    verify_artifact_names().unwrap();

    let version = workspace_version().unwrap();
    let deb = root.join(format!("GooglePicz-{}.deb", version));
    assert!(deb.exists());
    fs::remove_file(deb).unwrap();

    std::env::remove_var("MOCK_COMMANDS");
    std::env::remove_var("LINUX_PACKAGE_FORMAT");
}

#[cfg(target_os = "macos")]
#[test]
#[serial]
fn test_create_installer_macos_mock() {
    std::env::set_var("MOCK_COMMANDS", "1");
    std::env::set_var("MAC_SIGN_ID", "test");
    let root = get_project_root();
    let release_dir = root.join("target/release");
    fs::create_dir_all(&release_dir).unwrap();
    fs::write(release_dir.join("GooglePicz.dmg"), b"test").unwrap();

    let result = create_installer();
    assert!(result.is_ok());
    verify_artifact_names().unwrap();

    let version = workspace_version().unwrap();
    let dmg = release_dir.join(format!("GooglePicz-{}.dmg", version));
    assert!(dmg.exists());
    fs::remove_file(dmg).unwrap();

    std::env::remove_var("MOCK_COMMANDS");
    std::env::remove_var("MAC_SIGN_ID");
}

#[cfg(target_os = "windows")]
#[test]
#[serial]
fn test_create_installer_windows_mock() {
    std::env::set_var("MOCK_COMMANDS", "1");
    std::env::set_var("WINDOWS_CERT", "C:/dummy.pfx");
    std::env::set_var("WINDOWS_CERT_PASSWORD", "pw");
    let root = get_project_root();
    let win_dir = root.join("target/windows");
    fs::create_dir_all(&win_dir).unwrap();
    let version = workspace_version().unwrap();
    fs::write(win_dir.join(format!("GooglePicz-{}-Setup.exe", version)), b"test").unwrap();
    let rel_dir = root.join("target/release");
    fs::create_dir_all(&rel_dir).unwrap();
    fs::write(rel_dir.join("googlepicz.exe"), b"test").unwrap();

    let result = create_installer();
    assert!(result.is_ok());
    verify_artifact_names().unwrap();

    let exe = win_dir.join(format!("GooglePicz-{}-Setup.exe", version));
    assert!(exe.exists());
    fs::remove_file(exe).unwrap();
    fs::remove_file(rel_dir.join("googlepicz.exe")).unwrap();

    std::env::remove_var("MOCK_COMMANDS");
    std::env::remove_var("WINDOWS_CERT");
    std::env::remove_var("WINDOWS_CERT_PASSWORD");
}


use packaging::create_installer;
use packaging::utils::{get_project_root, workspace_version};
use serial_test::serial;
use std::fs;

#[cfg(target_os = "linux")]
#[test]
#[serial]
fn test_linux_installer_artifact_exists() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("MOCK_COMMANDS", "1");
    std::env::set_var("LINUX_PACKAGE_FORMAT", "deb");
    let root = get_project_root();
    let deb_dir = root.join("target/debian");
    fs::create_dir_all(&deb_dir)?;
    fs::write(deb_dir.join("dummy.deb"), b"test")?;

    create_installer()?;
    let version = workspace_version()?;
    let deb = root.join(format!("GooglePicz-{}.deb", version));
    assert!(deb.exists());
    fs::remove_file(deb)?;

    std::env::remove_var("MOCK_COMMANDS");
    std::env::remove_var("LINUX_PACKAGE_FORMAT");
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
#[serial]
fn test_macos_installer_artifact_exists() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("MOCK_COMMANDS", "1");
    let root = get_project_root();
    let release = root.join("target/release");
    fs::create_dir_all(&release)?;
    fs::write(release.join("GooglePicz.dmg"), b"test")?;
    let bundle_dir = release.join("bundle/osx/GooglePicz.app");
    fs::create_dir_all(&bundle_dir)?;

    create_installer()?;
    let version = workspace_version()?;
    let dmg = release.join(format!("GooglePicz-{}.dmg", version));
    assert!(dmg.exists());
    fs::remove_file(dmg)?;

    std::env::remove_var("MOCK_COMMANDS");
    Ok(())
}

#[cfg(target_os = "windows")]
#[test]
#[serial]
fn test_windows_installer_artifact_exists() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("MOCK_COMMANDS", "1");
    let root = get_project_root();
    let win_dir = root.join("target/windows");
    fs::create_dir_all(&win_dir)?;
    let version = workspace_version()?;
    fs::write(win_dir.join(format!("GooglePicz-{}-Setup.exe", version)), b"test")?;
    let rel_dir = root.join("target/release");
    fs::create_dir_all(&rel_dir)?;
    fs::write(rel_dir.join("googlepicz.exe"), b"test")?;

    create_installer()?;
    let exe = win_dir.join(format!("GooglePicz-{}-Setup.exe", version));
    assert!(exe.exists());
    fs::remove_file(exe)?;
    fs::remove_file(rel_dir.join("googlepicz.exe"))?;

    std::env::remove_var("MOCK_COMMANDS");
    Ok(())
}

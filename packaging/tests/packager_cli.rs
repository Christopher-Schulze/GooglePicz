use assert_cmd::Command;
use packaging::utils::{artifact_path, get_project_root, workspace_version};
use serial_test::serial;
use std::fs;

#[test]
#[serial]
fn test_packager_cli_format() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("MOCK_COMMANDS", "1");
    std::env::set_var("LINUX_PACKAGE_FORMAT", "rpm");
    let root = get_project_root();

    #[cfg(target_os = "linux")]
    {
        let rpm_dir = root.join("target/rpmbuild/RPMS");
        fs::create_dir_all(&rpm_dir)?;
        fs::write(rpm_dir.join("dummy.rpm"), b"test")?;
    }

    Command::cargo_bin("packager")?
        .arg("--format")
        .arg("rpm")
        .assert()
        .success();

    #[cfg(target_os = "linux")]
    {
        let version = workspace_version()?;
        let rpm = artifact_path(&version);
        assert!(rpm.exists());
        fs::remove_file(rpm)?;
    }

    std::env::remove_var("MOCK_COMMANDS");
    std::env::remove_var("LINUX_PACKAGE_FORMAT");
    Ok(())
}

#[test]
#[serial]
fn test_packager_cli_appimage() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("MOCK_COMMANDS", "1");
    std::env::set_var("LINUX_PACKAGE_FORMAT", "appimage");
    let root = get_project_root();

    #[cfg(target_os = "linux")]
    {
        let img_dir = root.join("target/appimage");
        fs::create_dir_all(&img_dir)?;
        fs::write(img_dir.join("dummy.AppImage"), b"test")?;
    }

    Command::cargo_bin("packager")?
        .arg("--format")
        .arg("appimage")
        .assert()
        .success();

    #[cfg(target_os = "linux")]
    {
        let version = workspace_version()?;
        let img = artifact_path(&version);
        assert!(img.exists());
        fs::remove_file(img)?;
    }

    std::env::remove_var("MOCK_COMMANDS");
    std::env::remove_var("LINUX_PACKAGE_FORMAT");
    Ok(())
}

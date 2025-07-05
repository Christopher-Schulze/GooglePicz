use packaging::package_all;
use packaging::utils::{get_project_root, workspace_version};
use serial_test::serial;
use std::fs;


#[test]
#[serial]
fn test_package_all_mock() {
    let use_real = std::env::var("CI_PACKAGING_TOOLS").is_ok();
    if !use_real {
        std::env::set_var("MOCK_COMMANDS", "1");
    }
    let root = get_project_root();

    if cfg!(target_os = "linux") {
        let deb_dir = root.join("target/debian");
        fs::create_dir_all(&deb_dir).unwrap();
        fs::write(deb_dir.join("dummy.deb"), b"test").unwrap();
    }

    if cfg!(target_os = "macos") {
        let release_dir = root.join("target/release");
        fs::create_dir_all(&release_dir).unwrap();
        fs::write(release_dir.join("GooglePicz.dmg"), b"test").unwrap();
    }

    if cfg!(target_os = "windows") {
        let version = workspace_version().unwrap();
        let win_dir = root.join("target/windows");
        fs::create_dir_all(&win_dir).unwrap();
        fs::write(win_dir.join(format!("GooglePicz-{}-Setup.exe", version)), b"test").unwrap();
    }

    let result = package_all();
    assert!(result.is_ok(), "Packaging failed: {:?}", result.err());

    if cfg!(target_os = "linux") {
        let version = workspace_version().unwrap();
        let deb_file = root.join(format!("GooglePicz-{}.deb", version));
        assert!(deb_file.exists(), "Expected {:?} to exist", deb_file);
        fs::remove_file(deb_file).unwrap();
    }

    if cfg!(target_os = "macos") {
        let version = workspace_version().unwrap();
        let dmg = root.join(format!("target/release/GooglePicz-{}.dmg", version));
        assert!(dmg.exists(), "Expected {:?} to exist", dmg);
        fs::remove_file(dmg).unwrap();
    }

    if cfg!(target_os = "windows") {
        let version = workspace_version().unwrap();
        let exe = root.join(format!("target/windows/GooglePicz-{}-Setup.exe", version));
        assert!(exe.exists(), "Expected {:?} to exist", exe);
        fs::remove_file(exe).unwrap();
    }

    if !use_real {
        std::env::remove_var("MOCK_COMMANDS");
    }
}



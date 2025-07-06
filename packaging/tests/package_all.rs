use packaging::package_all;
use packaging::utils::{artifact_path, get_project_root, workspace_version};
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
        let format = std::env::var("LINUX_PACKAGE_FORMAT").unwrap_or_else(|_| "deb".into());
        if format == "rpm" {
            let rpm_dir = root.join("target/rpmbuild/RPMS");
            fs::create_dir_all(&rpm_dir).unwrap();
            fs::write(rpm_dir.join("dummy.rpm"), b"test").unwrap();
        } else if format == "appimage" {
            let img_dir = root.join("target/appimage");
            fs::create_dir_all(&img_dir).unwrap();
            fs::write(img_dir.join("dummy.AppImage"), b"test").unwrap();
        } else {
            let deb_dir = root.join("target/debian");
            fs::create_dir_all(&deb_dir).unwrap();
            fs::write(deb_dir.join("dummy.deb"), b"test").unwrap();
        }
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
        let file = artifact_path(&version);
        assert!(file.exists(), "Expected {:?} to exist", file);
        fs::remove_file(file).unwrap();
    }

    if cfg!(target_os = "macos") {
        let version = workspace_version().unwrap();
        let dmg = artifact_path(&version);
        assert!(dmg.exists(), "Expected {:?} to exist", dmg);
        fs::remove_file(dmg).unwrap();
    }

    if cfg!(target_os = "windows") {
        let version = workspace_version().unwrap();
        let exe = artifact_path(&version);
        assert!(exe.exists(), "Expected {:?} to exist", exe);
        fs::remove_file(exe).unwrap();
    }

    let checksums = root.join("checksums.txt");
    assert!(checksums.exists(), "Expected {:?} to exist", checksums);

    if !use_real {
        std::env::remove_var("MOCK_COMMANDS");
    }
}

#[cfg(target_os = "linux")]
#[test]
#[serial]
fn test_package_all_all_formats() {
    let formats = ["deb", "rpm", "appimage"];
    for fmt in &formats {
        std::env::set_var("MOCK_COMMANDS", "1");
        std::env::set_var("LINUX_PACKAGE_FORMAT", fmt);

        let root = get_project_root();
        // create dummy artifact for each format
        match *fmt {
            "rpm" => {
                let dir = root.join("target/rpmbuild/RPMS");
                fs::create_dir_all(&dir).unwrap();
                fs::write(dir.join("dummy.rpm"), b"test").unwrap();
            }
            "appimage" => {
                let dir = root.join("target/appimage");
                fs::create_dir_all(&dir).unwrap();
                fs::write(dir.join("dummy.AppImage"), b"test").unwrap();
            }
            _ => {
                let dir = root.join("target/debian");
                fs::create_dir_all(&dir).unwrap();
                fs::write(dir.join("dummy.deb"), b"test").unwrap();
            }
        }

        package_all().unwrap();

        let version = workspace_version().unwrap();
        let artifact = artifact_path(&version);
        assert!(artifact.exists());
        fs::remove_file(&artifact).unwrap();
        let _ = fs::remove_file(root.join("checksums.txt"));

        std::env::remove_var("MOCK_COMMANDS");
        std::env::remove_var("LINUX_PACKAGE_FORMAT");
    }
}



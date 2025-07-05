use packaging::utils::get_project_root;
use packaging::clean_artifacts;
use serial_test::serial;
use std::fs;

#[test]
#[serial]
fn test_clean_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    let root = get_project_root();

    #[cfg(target_os = "linux")]
    let path = root.join("GooglePicz-temp.deb");
    #[cfg(target_os = "macos")]
    let path = root.join("target/release/GooglePicz-temp.dmg");
    #[cfg(target_os = "windows")]
    let path = root.join("target/windows/GooglePicz-temp-Setup.exe");

    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, b"test")?;

    clean_artifacts()?;

    assert!(!path.exists(), "expected {:?} to be removed", path);
    Ok(())
}

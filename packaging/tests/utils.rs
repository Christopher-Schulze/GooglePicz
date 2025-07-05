use packaging::utils::{
    get_project_root,
    workspace_version,
    verify_metadata_package_name,
    verify_artifact_names,
    write_checksums,
};
use serial_test::serial;

#[test]
#[serial]
fn test_workspace_version_and_metadata() -> Result<(), Box<dyn std::error::Error>> {
    // workspace_version should parse the version from the top-level Cargo.toml
    let version = workspace_version()?;
    assert!(!version.is_empty(), "version should not be empty");

    // verify_metadata_package_name should find the googlepicz package
    verify_metadata_package_name("googlepicz")?;
    Ok(())
}

#[test]
#[serial]
fn test_verify_artifact_names() -> Result<(), Box<dyn std::error::Error>> {
    let version = workspace_version()?;
    let root = get_project_root();

    // create dummy artifact based on target OS
    #[cfg(target_os = "linux")]
    let path = root.join(format!("GooglePicz-{}.deb", version));
    #[cfg(target_os = "macos")]
    let path = root.join(format!("target/release/GooglePicz-{}.dmg", version));
    #[cfg(target_os = "windows")]
    let path = root.join(format!("target/windows/GooglePicz-{}-Setup.exe", version));

    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, b"test")?;

    verify_artifact_names()?;

    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
#[serial]
fn test_write_checksums() -> Result<(), Box<dyn std::error::Error>> {
    let version = workspace_version()?;
    let root = get_project_root();

    #[cfg(target_os = "linux")]
    let path = root.join(format!("GooglePicz-{}.deb", version));
    #[cfg(target_os = "macos")]
    let path = root.join(format!("target/release/GooglePicz-{}.dmg", version));
    #[cfg(target_os = "windows")]
    let path = root.join(format!("target/windows/GooglePicz-{}-Setup.exe", version));

    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, b"test")?;

    write_checksums()?;

    let checksums = std::fs::read_to_string(root.join("checksums.txt"))?;
    assert!(checksums.contains(path.file_name().unwrap().to_str().unwrap()));

    std::fs::remove_file(path)?;
    Ok(())
}

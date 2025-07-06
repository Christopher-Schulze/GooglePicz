use packaging::utils::{
    get_project_root,
    workspace_version,
    verify_metadata_package_name,
    verify_artifact_names,
    write_checksums,
    artifact_path,
};
use serial_test::serial;
use toml::Value;

#[test]
#[serial]
fn test_workspace_version_and_metadata() -> Result<(), Box<dyn std::error::Error>> {
    // workspace_version should match the version from the top-level Cargo.toml
    let version = workspace_version()?;
    let root = get_project_root();
    let toml_str = std::fs::read_to_string(root.join("Cargo.toml"))?;
    let value: Value = toml::from_str(&toml_str)?;
    let expected = value
        .get("workspace")
        .and_then(|ws| ws.get("package"))
        .and_then(|pkg| pkg.get("version"))
        .and_then(|v| v.as_str())
        .unwrap();
    assert_eq!(version, expected);

    // verify_metadata_package_name should find the googlepicz package
    verify_metadata_package_name("googlepicz")?;
    Ok(())
}

#[test]
#[serial]
fn test_get_project_root() -> Result<(), Box<dyn std::error::Error>> {
    let root = get_project_root();
    let cargo = root.join("Cargo.toml");
    assert!(cargo.exists());
    let contents = std::fs::read_to_string(cargo)?;
    assert!(contents.contains("[workspace]"));
    Ok(())
}

#[test]
#[serial]
fn test_verify_artifact_names() -> Result<(), Box<dyn std::error::Error>> {
    let version = workspace_version()?;
    let path = artifact_path(&version);

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
    let path = artifact_path(&version);
    let root = get_project_root();

    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, b"test")?;

    write_checksums()?;

    let checksums = std::fs::read_to_string(root.join("checksums.txt"))?;
    assert!(checksums.contains(path.file_name().unwrap().to_str().unwrap()));

    std::fs::remove_file(path)?;
    Ok(())
}

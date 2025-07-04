use std::fs;
use std::path::PathBuf;
use std::process::Command;

use toml::Value;
use serde_json::Value as JsonValue;

use crate::PackagingError;

/// Locate the workspace root by traversing up the directory tree
/// until a Cargo.toml containing `[workspace]` is found.
pub fn get_project_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            if let Ok(contents) = fs::read_to_string(&candidate) {
                if contents.contains("[workspace]") {
                    return dir;
                }
            }
        }
        if !dir.pop() {
            break;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Read `workspace.package.version` from the workspace Cargo.toml.
pub fn workspace_version() -> Result<String, PackagingError> {
    let cargo_toml = fs::read_to_string(get_project_root().join("Cargo.toml"))
        .map_err(|e| PackagingError::Other(format!("Failed to read Cargo.toml: {}", e)))?;
    let value: Value = toml::from_str(&cargo_toml)
        .map_err(|e| PackagingError::Other(format!("Failed to parse Cargo.toml: {}", e)))?;
    value
        .get("workspace")
        .and_then(|ws| ws.get("package"))
        .and_then(|pkg| pkg.get("version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| PackagingError::Other("workspace.package.version not found".into()))
}

/// Verify that `cargo metadata` lists the expected package name.
pub fn verify_metadata_package_name(expected: &str) -> Result<(), PackagingError> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .map_err(|e| PackagingError::Other(format!("Failed to run cargo metadata: {}", e)))?;
    if !output.status.success() {
        return Err(PackagingError::Other("cargo metadata failed".into()));
    }
    let metadata: JsonValue = serde_json::from_slice(&output.stdout)
        .map_err(|e| PackagingError::Other(format!("Failed to parse cargo metadata: {}", e)))?;
    let packages = metadata
        .get("packages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| PackagingError::Other("No packages field in metadata".into()))?;

    let found = packages.iter().any(|pkg| pkg.get("name").and_then(|n| n.as_str()) == Some(expected));
    if found {
        Ok(())
    } else {
        Err(PackagingError::Other(format!("Package '{}' not found", expected)))
    }
}

/// Check that the built installer artifacts include the workspace version in their name.
pub fn verify_artifact_names() -> Result<(), PackagingError> {
    let root = get_project_root();
    let version = workspace_version()?;

    if cfg!(target_os = "linux") {
        let path = root.join(format!("GooglePicz-{}.deb", version));
        if !path.exists() {
            return Err(PackagingError::Other(format!("Missing artifact: {:?}", path)));
        }
    } else if cfg!(target_os = "macos") {
        let path = root.join(format!("target/release/GooglePicz-{}.dmg", version));
        if !path.exists() {
            return Err(PackagingError::Other(format!("Missing artifact: {:?}", path)));
        }
    } else if cfg!(target_os = "windows") {
        let path = root.join(format!("target/windows/GooglePicz-{}-Setup.exe", version));
        if !path.exists() {
            return Err(PackagingError::Other(format!("Missing artifact: {:?}", path)));
        }
    }

    Ok(())
}

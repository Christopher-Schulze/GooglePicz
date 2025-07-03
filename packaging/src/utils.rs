use std::fs;
use std::path::PathBuf;

use toml::Value;

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

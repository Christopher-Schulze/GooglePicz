//! Packaging module for GooglePicz.

use std::process::Command;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum PackagingError {
    CommandError(String),
    Other(String),
}

impl fmt::Display for PackagingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PackagingError::CommandError(msg) => write!(f, "Command Error: {}", msg),
            PackagingError::Other(msg) => write!(f, "Other Error: {}", msg),
        }
    }
}

impl Error for PackagingError {}

pub fn bundle_licenses() -> Result<(), PackagingError> {
    println!("Bundling licenses...");
    let output = Command::new("cargo")
        .args(&["bundle-licenses", "--format", "json", "--output", "licenses.json"])
        .output()
        .map_err(|e| PackagingError::CommandError(format!("Failed to execute cargo bundle-licenses: {}", e)))?;

    if output.status.success() {
        println!("Licenses bundled successfully.");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(PackagingError::CommandError(format!("cargo bundle-licenses failed: {}", stderr)))
    }
}

pub fn build_release() -> Result<(), PackagingError> {
    println!("Building release binary...");
    let output = Command::new("cargo")
        .args(&["build", "--release"])
        .output()
        .map_err(|e| PackagingError::CommandError(format!("Failed to execute cargo build --release: {}", e)))?;

    if output.status.success() {
        println!("Release binary built successfully.");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(PackagingError::CommandError(format!("cargo build --release failed: {}", stderr)))
    }
}

// This function would typically be more complex, involving platform-specific tools
// like `cargo-bundle` for macOS .app bundles, or NSIS for Windows installers.
// For now, it's a placeholder.
pub fn create_installer() -> Result<(), PackagingError> {
    println!("Creating installer (placeholder)...");
    // Example: For macOS, you might use cargo-bundle or a custom script
    // Command::new("cargo").args(&["bundle", "--release"]).output();
    // For Windows, you might use NSIS or WiX
    // For Linux, .deb or .rpm packages
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    // Helper to find the project root (where Cargo.toml is)
    fn get_project_root() -> PathBuf {
        let mut current_dir = std::env::current_dir().unwrap();
        while !current_dir.join("Cargo.toml").exists() {
            current_dir.pop();
        }
        current_dir
    }

    #[test]
    #[ignore] // This test actually runs cargo commands, can be slow and requires cargo-bundle-licenses
    fn test_bundle_licenses() {
        let project_root = get_project_root();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_root).unwrap();

        let result = bundle_licenses();
        assert!(result.is_ok(), "License bundling failed: {:?}", result.err());

        let licenses_file = project_root.join("licenses.json");
        assert!(licenses_file.exists());
        fs::remove_file(licenses_file).unwrap(); // Clean up

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    #[ignore] // This test actually runs cargo commands, can be slow
    fn test_build_release() {
        let project_root = get_project_root();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_root).unwrap();

        let result = build_release();
        assert!(result.is_ok(), "Release build failed: {:?}", result.err());

        // Check if the release binary exists (platform-dependent)
        let target_dir = project_root.join("target").join("release");
        let binary_name = if cfg!(target_os = "windows") {
            "googlepicz.exe"
        } else {
            "googlepicz"
        };
        assert!(target_dir.join(binary_name).exists());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_create_installer() {
        // This is a placeholder test for a placeholder function.
        // In a real scenario, this would involve more complex setup and assertions.
        let result = create_installer();
        assert!(result.is_ok());
    }
}
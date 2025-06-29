//! Packaging module for GooglePicz.

use std::error::Error;
use std::fmt;
use std::process::Command;

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

fn run_command(cmd: &str, args: &[&str]) -> Result<(), PackagingError> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| PackagingError::CommandError(format!("Failed to execute {}: {}", cmd, e)))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(PackagingError::CommandError(format!("{} failed: {}", cmd, stderr)))
    }
}

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

fn create_macos_installer() -> Result<(), PackagingError> {
    println!("Bundling macOS app...");
    run_command("cargo", &["bundle", "--release"])?;

    println!("Signing macOS app...");
    let identity = std::env::var("MAC_SIGN_ID").unwrap_or_default();
    let app_path = "target/release/bundle/osx/GooglePicz.app";
    if !identity.is_empty() {
        run_command("codesign", &["--deep", "--force", "-s", &identity, app_path])?;
    }

    if std::env::var("APPLE_ID").is_ok() {
        let apple_id = std::env::var("APPLE_ID").unwrap();
        let password = std::env::var("APPLE_PASSWORD").unwrap_or_default();
        run_command(
            "xcrun",
            &["notarytool", "submit", app_path, "--apple-id", &apple_id, "--password", &password, "--wait"],
        )?;
    }

    Ok(())
}

fn create_windows_installer() -> Result<(), PackagingError> {
    println!("Creating Windows installer...");
    run_command("makensis", &["packaging/installer.nsi"])
}

pub fn create_installer() -> Result<(), PackagingError> {
    if cfg!(target_os = "macos") {
        create_macos_installer()
    } else if cfg!(target_os = "windows") {
        create_windows_installer()
    } else {
        println!("Installer creation not supported on this OS");
        Ok(())
    }
}

pub fn package_all() -> Result<(), PackagingError> {
    bundle_licenses()?;
    build_release()?;
    create_installer()
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


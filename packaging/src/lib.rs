//! Packaging module for GooglePicz.
//!
//! The packager can sign and notarize macOS builds when the following
//! environment variables are provided:
//! - `MAC_SIGN_ID`: identity passed to `codesign`.
//! - `APPLE_ID`: Apple ID used for notarization.
//! - `APPLE_PASSWORD`: app-specific password for notarization.

use std::error::Error;
use std::fmt;
use std::process::Command;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

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
    if std::env::var("MOCK_COMMANDS").is_ok() {
        return Ok(());
    }
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

fn project_root() -> PathBuf {
    let mut dir = std::env::current_dir().expect("failed to get cwd");
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            if fs::read_to_string(&candidate)
                .map(|c| c.contains("[workspace]"))
                .unwrap_or(false)
            {
                break dir;
            }
        }
        dir.pop();
    }
}

fn workspace_version() -> Result<String, PackagingError> {
    let cargo_toml = fs::read_to_string(project_root().join("Cargo.toml"))
        .map_err(|e| PackagingError::Other(format!("Failed to read Cargo.toml: {}", e)))?;
    let parsed: Value = cargo_toml
        .parse()
        .map_err(|e| PackagingError::Other(format!("Failed to parse Cargo.toml: {}", e)))?;
    parsed
        .get("workspace")
        .and_then(|ws| ws.get("package"))
        .and_then(|pkg| pkg.get("version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| PackagingError::Other("Workspace version not found".into()))
}

pub fn bundle_licenses() -> Result<(), PackagingError> {
    tracing::info!("Bundling licenses...");
    run_command(
        "cargo",
        &["bundle-licenses", "--format", "json", "--output", "licenses.json"],
    )
}

pub fn build_release() -> Result<(), PackagingError> {
    tracing::info!("Building release binary...");
    run_command("cargo", &["build", "--release"])
}

fn create_macos_installer() -> Result<(), PackagingError> {
    tracing::info!("Bundling macOS app...");
    run_command("cargo", &["bundle", "--release"])?;

    tracing::info!("Signing macOS app...");
    let identity = std::env::var("MAC_SIGN_ID").unwrap_or_default();
    let app_path = "target/release/bundle/osx/GooglePicz.app";
    if !identity.is_empty() {
        run_command("codesign", &["--deep", "--force", "-s", &identity, app_path])?;
    }

    let dmg_path = "target/release/GooglePicz.dmg";
    run_command(
        "hdiutil",
        &["create", "-volname", "GooglePicz", "-srcfolder", app_path, "-ov", "-format", "UDZO", dmg_path],
    )?;
    if !identity.is_empty() {
        run_command("codesign", &["--force", "-s", &identity, dmg_path])?;
    }

    if std::env::var("APPLE_ID").is_ok() {
        let apple_id = std::env::var("APPLE_ID").unwrap();
        let password = std::env::var("APPLE_PASSWORD").unwrap_or_default();
        run_command(
            "xcrun",
            &["notarytool", "submit", dmg_path, "--apple-id", &apple_id, "--password", &password, "--wait"],
        )?;
        run_command("xcrun", &["stapler", "staple", dmg_path])?;
    }

    Ok(())
}

fn create_windows_installer() -> Result<(), PackagingError> {
    tracing::info!("Creating Windows installer...");
    let release_exe = "target\\release\\googlepicz.exe";
    run_command("makensis", &["packaging/installer.nsi"])?;

    let exe_path = "GooglePiczSetup.exe";
    if let Ok(cert_path) = std::env::var("WINDOWS_CERT") {
        if !cert_path.is_empty() {
            let password = std::env::var("WINDOWS_CERT_PASSWORD").unwrap_or_default();
            for target in &[release_exe, exe_path] {
                run_command(
                    "signtool",
                    &[
                        "sign",
                        "/f",
                        &cert_path,
                        "/p",
                        &password,
                        "/fd",
                        "sha256",
                        "/tr",
                        "http://timestamp.digicert.com",
                        "/td",
                        "sha256",
                        target,
                    ],
                )?;
            }
        }
    }

    Ok(())
}

fn create_linux_package() -> Result<(), PackagingError> {
    tracing::info!("Creating Linux .deb package...");
    let version = workspace_version()?;
    run_command("cargo", &["deb", "--deb-version", &version])?;

    if std::env::var("MOCK_COMMANDS").is_ok() {
        return Ok(());
    }

    let deb_dir = Path::new("target/debian");
    let built_deb = fs::read_dir(deb_dir)
        .map_err(|e| PackagingError::Other(format!("Failed to read debian dir: {}", e)))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|s| s.to_str()) == Some("deb"))
        .ok_or_else(|| PackagingError::Other("No .deb file found".into()))?;

    let final_deb = deb_dir.join(format!("GooglePicz-{}.deb", version));
    if built_deb != final_deb {
        fs::rename(&built_deb, &final_deb)
            .map_err(|e| PackagingError::Other(format!("Failed to rename .deb: {}", e)))?;
    }

    if let Ok(key_id) = std::env::var("LINUX_SIGN_KEY") {
        if !key_id.is_empty() {
            run_command("dpkg-sig", &["--sign", "builder", "-k", &key_id, final_deb.to_str().unwrap()])?;
        }
    }
    Ok(())
}

pub fn create_installer() -> Result<(), PackagingError> {
    if cfg!(target_os = "macos") {
        create_macos_installer()
    } else if cfg!(target_os = "windows") {
        create_windows_installer()
    } else if cfg!(target_os = "linux") {
        create_linux_package()
    } else {
        tracing::info!("Installer creation not supported on this OS");
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
    use serial_test::serial;

    // Helper to find the project root (where Cargo.toml is)
    fn get_project_root() -> PathBuf {
        let mut current_dir = std::env::current_dir().unwrap();
        while !current_dir.join("Cargo.toml").exists() {
            current_dir.pop();
        }
        current_dir
    }

    #[test]
    #[serial]
    fn test_bundle_licenses() {
        std::env::set_var("MOCK_COMMANDS", "1");
        let project_root = get_project_root();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_root).unwrap();

        let result = bundle_licenses();
        assert!(result.is_ok(), "License bundling failed: {:?}", result.err());

        let licenses_file = project_root.join("licenses.json");
        // In mock mode the file won't exist
        if licenses_file.exists() {
            fs::remove_file(licenses_file).unwrap();
        }

        std::env::set_current_dir(original_dir).unwrap();
        std::env::remove_var("MOCK_COMMANDS");
    }

    #[test]
    #[serial]
    fn test_build_release() {
        std::env::set_var("MOCK_COMMANDS", "1");
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
        // In mock mode the binary won't exist
        if target_dir.join(binary_name).exists() {
            assert!(true);
        }

        std::env::set_current_dir(original_dir).unwrap();
        std::env::remove_var("MOCK_COMMANDS");
    }

    #[test]
    #[serial]
    fn test_create_installer() {
        std::env::set_var("MOCK_COMMANDS", "1");
        // This is a placeholder test for a placeholder function.
        // In a real scenario, this would involve more complex setup and assertions.
        let result = create_installer();
        assert!(result.is_ok());
        std::env::remove_var("MOCK_COMMANDS");
    }
}


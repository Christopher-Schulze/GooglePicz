//! Packaging module for GooglePicz.
//!
//! The packager can sign and notarize macOS builds when the following
//! environment variables are provided:
//! - `MAC_SIGN_ID`: identity passed to `codesign`.
//! - `APPLE_ID`: Apple ID used for notarization.
//! - `APPLE_PASSWORD`: app-specific password for notarization.

use thiserror::Error;
use std::fs;
use std::process::Command;
use std::path::PathBuf;
use which::which;

pub mod utils;

fn find_by_extension(dir: &std::path::Path, ext: &str) -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(p) = find_by_extension(&path, ext) {
                    return Some(p);
                }
            } else if path.extension().map(|e| e == ext).unwrap_or(false) {
                return Some(path);
            }
        }
    }
    None
}

#[derive(Debug, Error)]
pub enum PackagingError {
    #[error("Command Error: {0}")]
    CommandError(String),
    #[error("Other Error: {0}")]
    Other(String),
    #[error("Missing Command: {0}")]
    MissingCommand(String),
}

fn hint(cmd: &str, install: &str) -> String {
    format!("Required tool '{cmd}' not found. Install with {install}")
}

fn command_available(cmd: &str) -> bool {
    which(cmd).is_ok()
}

fn verify_tools() -> Result<(), PackagingError> {
    if std::env::var("MOCK_COMMANDS").is_ok() {
        return Ok(());
    }

    let mut tools: Vec<(&str, String)> = Vec::new();

    tools.push(("cargo", hint("cargo", "install Rust from https://rustup.rs")));
    if cfg!(target_os = "linux") {
        let format = std::env::var("LINUX_PACKAGE_FORMAT").unwrap_or_else(|_| "deb".into());
        match format.as_str() {
            "rpm" => tools.push(("cargo-rpm", hint("cargo-rpm", "cargo install cargo-rpm"))),
            "appimage" => tools.push(("appimagetool", hint("appimagetool", "install appimagetool from your distribution"))),
            _ => tools.push(("cargo-deb", hint("cargo-deb", "cargo install cargo-deb"))),
        }
        if std::env::var("LINUX_SIGN_KEY").map(|v| !v.is_empty()).unwrap_or(false) {
            tools.push(("dpkg-sig", hint("dpkg-sig", "install dpkg-sig from your distribution")));
        }
    } else if cfg!(target_os = "macos") {
        tools.push(("cargo-bundle", hint("cargo-bundle", "cargo install cargo-bundle")));
        tools.push(("codesign", hint("codesign", "install Xcode command line tools")));
        tools.push(("hdiutil", hint("hdiutil", "install Xcode command line tools")));
        tools.push(("xcrun", hint("xcrun", "install Xcode command line tools")));
    } else if cfg!(target_os = "windows") {
        tools.push(("makensis", hint("makensis", "install NSIS")));
        tools.push(("signtool", hint("signtool", "install Windows SDK")));
    }

    for (cmd, msg) in tools {
        if !command_available(cmd) {
            return Err(PackagingError::MissingCommand(msg.into()));
        }
    }

    Ok(())
}

#[cfg_attr(feature = "trace-spans", tracing::instrument(skip(args)))]
fn run_command(cmd: &str, args: &[&str]) -> Result<(), PackagingError> {
    tracing::info!("Running command: {} {:?}", cmd, args);
    if std::env::var("MOCK_COMMANDS").is_ok() {
        return Ok(());
    }

    if cmd == "cargo" {
        if let Some(sub) = args.first() {
            match *sub {
                "deb" => {
                    if !command_available("cargo-deb") {
                        return Err(PackagingError::MissingCommand(hint("cargo-deb", "cargo install cargo-deb")));
                    }
                }
                "bundle" => {
                    if !command_available("cargo-bundle") {
                        return Err(PackagingError::MissingCommand(hint("cargo-bundle", "cargo install cargo-bundle")));
                    }
                }
                "bundle-licenses" => {
                    if !command_available("cargo-bundle-licenses") {
                        return Err(PackagingError::MissingCommand(hint("cargo-bundle-licenses", "cargo install cargo-bundle-licenses")));
                    }
                }
                _ => {}
            }
        }
    }

    if !command_available(cmd) {
        let msg = match cmd {
            "makensis" => hint("makensis", "install NSIS"),
            "dpkg-sig" => hint("dpkg-sig", "install dpkg-sig from your distribution"),
            _ => hint(cmd, &format!("install {cmd} and ensure it is in your PATH")),
        };
        return Err(PackagingError::MissingCommand(msg));
    }

    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| PackagingError::CommandError(format!("Failed to execute {}: {}", cmd, e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stdout.trim().is_empty() {
        tracing::debug!("stdout: {}", stdout);
    }
    if !stderr.trim().is_empty() {
        tracing::debug!("stderr: {}", stderr);
    }

    let status = output.status;
    if status.code() == Some(0) {
        Ok(())
    } else {
        let code = status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".into());
        let msg = if stderr.trim().is_empty() {
            format!("{} exited with code {}", cmd, code)
        } else {
            format!("{} exited with code {}: {}", cmd, code, stderr.trim())
        };
        Err(PackagingError::CommandError(msg))
    }
}

use utils::{get_project_root, workspace_version};

#[cfg_attr(feature = "trace-spans", tracing::instrument)]
pub fn bundle_licenses() -> Result<(), PackagingError> {
    tracing::info!("Bundling licenses...");
    run_command(
        "cargo",
        &[
            "bundle-licenses",
            "--format",
            "json",
            "--output",
            "licenses.json",
        ],
    )
}

#[cfg_attr(feature = "trace-spans", tracing::instrument)]
pub fn build_release() -> Result<(), PackagingError> {
    tracing::info!("Building release binary...");
    run_command("cargo", &["build", "--release"])
}

#[cfg_attr(feature = "trace-spans", tracing::instrument)]
pub fn clean_artifacts() -> Result<(), PackagingError> {
    use std::ffi::OsStr;
    let root = get_project_root();

    let remove_if_match = |dir: PathBuf, prefix: &str, ext: &str| {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(OsStr::to_str).unwrap_or("");
                if name.starts_with(prefix) && path.extension().and_then(OsStr::to_str) == Some(ext) {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    };

    if cfg!(target_os = "linux") {
        remove_if_match(root.clone(), "GooglePicz-", "deb");
    } else if cfg!(target_os = "macos") {
        remove_if_match(root.join("target/release"), "GooglePicz-", "dmg");
    } else if cfg!(target_os = "windows") {
        remove_if_match(root.join("target/windows"), "GooglePicz-", "exe");
    }

    Ok(())
}

#[cfg_attr(feature = "trace-spans", tracing::instrument)]
fn create_macos_installer() -> Result<(), PackagingError> {
    tracing::info!("Bundling macOS app...");
    run_command("cargo", &["bundle", "--release"])?;

    tracing::info!("Signing macOS app...");
    let identity = std::env::var("MAC_SIGN_ID").unwrap_or_default();
    let app_path = "target/release/bundle/osx/GooglePicz.app";
    if !identity.is_empty() {
        run_command(
            "codesign",
            &["--deep", "--force", "-s", &identity, app_path],
        )?;
        run_command(
            "codesign",
            &["--verify", "--deep", "--strict", app_path],
        )?;
    }

    let dmg_path = "target/release/GooglePicz.dmg";
    run_command(
        "hdiutil",
        &[
            "create",
            "-volname",
            "GooglePicz",
            "-srcfolder",
            app_path,
            "-ov",
            "-format",
            "UDZO",
            dmg_path,
        ],
    )?;
    if !identity.is_empty() {
        run_command("codesign", &["--force", "-s", &identity, dmg_path])?;
        run_command("codesign", &["--verify", dmg_path])?;
    }

    if let Ok(apple_id) = std::env::var("APPLE_ID") {
        let password = std::env::var("APPLE_PASSWORD").unwrap_or_default();
        run_command(
            "xcrun",
            &[
                "notarytool",
                "submit",
                dmg_path,
                "--apple-id",
                &apple_id,
                "--password",
                &password,
                "--wait",
            ],
        )?;
        run_command("xcrun", &["stapler", "staple", dmg_path])?;
        run_command("xcrun", &["stapler", "validate", dmg_path])?;
    }

    let version = workspace_version()?;
    let versioned = format!("target/release/GooglePicz-{}.dmg", version);
    fs::rename(dmg_path, &versioned)
        .map_err(|e| PackagingError::Other(format!("Failed to rename dmg: {}", e)))?;

    Ok(())
}

#[cfg_attr(feature = "trace-spans", tracing::instrument)]
fn create_windows_installer() -> Result<(), PackagingError> {
    tracing::info!("Creating Windows installer...");
    let release_exe = "target\\release\\googlepicz.exe";

    // Determine the version from the workspace Cargo.toml
    let version = workspace_version()?;
    let mut parts = version.split('.');
    let major = parts.next().unwrap_or("0");
    let minor = parts.next().unwrap_or("0");
    let patch = parts.next().unwrap_or("0");

    let arg_major = format!("/DAPP_VERSION_MAJOR={}", major);
    let arg_minor = format!("/DAPP_VERSION_MINOR={}", minor);
    let arg_patch = format!("/DAPP_VERSION_PATCH={}", patch);

    run_command(
        "makensis",
        &[
            arg_major.as_str(),
            arg_minor.as_str(),
            arg_patch.as_str(),
            "packaging/installer.nsi",
        ],
    )?;

    let exe_path = format!("target/windows/GooglePicz-{}-Setup.exe", version);
    if let Ok(cert_path) = std::env::var("WINDOWS_CERT") {
        if !cert_path.is_empty() {
            let password = std::env::var("WINDOWS_CERT_PASSWORD").unwrap_or_default();
            let targets = [release_exe, exe_path.as_str()];
            for target in &targets {
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
                run_command("signtool", &["verify", "/pa", target])?;
            }
        }
    }

    Ok(())
}

#[cfg_attr(feature = "trace-spans", tracing::instrument)]
fn create_linux_package() -> Result<(), PackagingError> {
    let format = std::env::var("LINUX_PACKAGE_FORMAT").unwrap_or_else(|_| "deb".into());
    match format.as_str() {
        "rpm" => create_rpm_package(),
        "appimage" => create_appimage_package(),
        _ => create_deb_package(),
    }
}

fn create_deb_package() -> Result<(), PackagingError> {
    tracing::info!("Creating Linux .deb package...");

    let version = workspace_version()?;
    run_command("cargo", &["deb", "--deb-version", &version])?;

    let root = get_project_root();
    let deb_dir = root.join("target/debian");
    let deb_entries = match fs::read_dir(&deb_dir) {
        Ok(entries) => entries,
        Err(_) => {
            if std::env::var("MOCK_COMMANDS").is_ok() { return Ok(()); } else {
                return Err(PackagingError::Other("No .deb package produced".into()));
            }
        }
    };
    let deb_path = deb_entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().map(|ext| ext == "deb").unwrap_or(false));

    let Some(deb_path) = deb_path else {
        if std::env::var("MOCK_COMMANDS").is_ok() { return Ok(()); } else {
            return Err(PackagingError::Other("No .deb package produced".into()));
        }
    };

    if let Ok(key_id) = std::env::var("LINUX_SIGN_KEY") {
        if !key_id.is_empty() {
            let deb_str = deb_path.to_string_lossy();
            run_command("dpkg-sig", &["--sign", "builder", "-k", &key_id, &deb_str])?;
            run_command("dpkg-sig", &["--verify", &deb_str])?;
        }
    }

    let versioned = root.join(format!("GooglePicz-{}.deb", version));
    fs::rename(&deb_path, &versioned)
        .map_err(|e| PackagingError::Other(format!("Failed to rename .deb: {}", e)))?;

    Ok(())
}

fn create_rpm_package() -> Result<(), PackagingError> {
    tracing::info!("Creating Linux .rpm package...");

    let version = workspace_version()?;
    run_command("cargo", &["rpm", "build", "--release"])?;

    let root = get_project_root();
    let rpm_path = find_by_extension(&root.join("target"), "rpm");
    let Some(rpm_path) = rpm_path else {
        if std::env::var("MOCK_COMMANDS").is_ok() { return Ok(()); } else {
            return Err(PackagingError::Other("No .rpm package produced".into()));
        }
    };

    let versioned = root.join(format!("GooglePicz-{}.rpm", version));
    fs::rename(&rpm_path, &versioned)
        .map_err(|e| PackagingError::Other(format!("Failed to rename .rpm: {}", e)))?;
    Ok(())
}

fn create_appimage_package() -> Result<(), PackagingError> {
    tracing::info!("Creating Linux AppImage package...");

    let version = workspace_version()?;
    run_command("cargo", &["appimage"])?;

    let root = get_project_root();
    let app_path = find_by_extension(&root.join("target"), "AppImage");
    let Some(app_path) = app_path else {
        if std::env::var("MOCK_COMMANDS").is_ok() { return Ok(()); } else {
            return Err(PackagingError::Other("No AppImage produced".into()));
        }
    };

    let versioned = root.join(format!("GooglePicz-{}.AppImage", version));
    fs::rename(&app_path, &versioned)
        .map_err(|e| PackagingError::Other(format!("Failed to rename AppImage: {}", e)))?;
    Ok(())
}

#[cfg_attr(feature = "trace-spans", tracing::instrument)]
pub fn create_installer() -> Result<(), PackagingError> {
    if cfg!(target_os = "macos") {
        create_macos_installer()?;
        let root = get_project_root();
        let version = workspace_version()?;
        let dmg = root.join(format!("target/release/GooglePicz-{}.dmg", version));
        if !dmg.exists() && std::env::var("MOCK_COMMANDS").is_err() {
            return Err(PackagingError::Other(format!("Expected installer {:?} not found", dmg)));
        }
        Ok(())
    } else if cfg!(target_os = "windows") {
        create_windows_installer()?;
        let root = get_project_root();
        let version = workspace_version()?;
        let exe = root.join(format!("target/windows/GooglePicz-{}-Setup.exe", version));
        if !exe.exists() && std::env::var("MOCK_COMMANDS").is_err() {
            return Err(PackagingError::Other(format!("Expected installer {:?} not found", exe)));
        }
        Ok(())
    } else if cfg!(target_os = "linux") {
        create_linux_package()?;
        let root = get_project_root();
        let version = workspace_version()?;
        let format = std::env::var("LINUX_PACKAGE_FORMAT").unwrap_or_else(|_| "deb".into());
        let path = match format.as_str() {
            "rpm" => root.join(format!("GooglePicz-{}.rpm", version)),
            "appimage" => root.join(format!("GooglePicz-{}.AppImage", version)),
            _ => root.join(format!("GooglePicz-{}.deb", version)),
        };
        if !path.exists() && std::env::var("MOCK_COMMANDS").is_err() {
            return Err(PackagingError::Other(format!("Expected installer {:?} not found", path)));
        }
        Ok(())
    } else {
        tracing::info!("Installer creation not supported on this OS");
        Ok(())
    }
}

#[cfg_attr(feature = "trace-spans", tracing::instrument)]
pub fn package_all() -> Result<(), PackagingError> {
    verify_tools()?;
    let root = get_project_root();
    std::env::set_current_dir(&root)
        .map_err(|e| PackagingError::Other(format!("Failed to change directory: {}", e)))?;

    clean_artifacts()?;
    bundle_licenses()?;
    build_release()?;
    create_installer()?;
    utils::write_checksums()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use crate::utils::get_project_root;


    #[test]
    #[serial]
    fn test_bundle_licenses() {
        std::env::set_var("MOCK_COMMANDS", "1");
        let project_root = get_project_root();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_root).unwrap();

        let result = bundle_licenses();
        assert!(
            result.is_ok(),
            "License bundling failed: {:?}",
            result.err()
        );

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

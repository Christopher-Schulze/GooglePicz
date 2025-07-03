use packaging::package_all;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use toml::Value;

fn workspace_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap();
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
    std::env::current_dir().unwrap()
}

fn workspace_version(root: &PathBuf) -> String {
    let contents = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let value: Value = toml::from_str(&contents).unwrap();
    value
        .get("workspace")
        .and_then(|ws| ws.get("package"))
        .and_then(|pkg| pkg.get("version"))
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string()
}

#[test]
#[serial]
#[cfg(target_os = "linux")]
fn test_package_all_creates_deb() {
    std::env::set_var("MOCK_COMMANDS", "1");
    let root = workspace_root();
    let deb_dir = root.join("target/debian");
    fs::create_dir_all(&deb_dir).unwrap();
    fs::write(deb_dir.join("dummy.deb"), b"test").unwrap();

    let result = package_all();
    assert!(result.is_ok(), "Packaging failed: {:?}", result.err());

    let version = workspace_version(&root);
    let deb_file = root.join(format!("GooglePicz-{}.deb", version));
    assert!(deb_file.exists(), "Expected {:?} to exist", deb_file);
    fs::remove_file(deb_file).unwrap();
    std::env::remove_var("MOCK_COMMANDS");
}

#[test]
#[serial]
#[cfg(target_os = "windows")]
fn test_package_all_creates_exe() {
    std::env::set_var("MOCK_COMMANDS", "1");
    let root = workspace_root();
    let version = workspace_version(&root);
    let win_dir = root.join("target/windows");
    fs::create_dir_all(&win_dir).unwrap();
    fs::write(win_dir.join(format!("GooglePicz-{}-Setup.exe", version)), b"test").unwrap();

    let result = package_all();
    assert!(result.is_ok(), "Packaging failed: {:?}", result.err());

    let exe = root.join(format!("target/windows/GooglePicz-{}-Setup.exe", version));
    assert!(exe.exists(), "Expected {:?} to exist", exe);
    fs::remove_file(exe).unwrap();
    std::env::remove_var("MOCK_COMMANDS");
}


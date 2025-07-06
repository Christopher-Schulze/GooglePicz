use packaging::utils::verify_installer_tools;
use serial_test::serial;

#[test]
#[serial]
fn test_verify_installer_tools_mock_skips_missing() {
    let orig_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", "");
    std::env::set_var("MOCK_COMMANDS", "1");
    let result = verify_installer_tools();
    assert!(result.is_ok(), "expected success when MOCK_COMMANDS is set");
    std::env::remove_var("MOCK_COMMANDS");
    std::env::set_var("PATH", orig_path);
}

use escargot::CargoBuild;
use tempfile::TempDir;

#[test]
fn test_remove_nonexistent_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let vex_bin = CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(&["rm", "nonexistent"])
        .output()
        .unwrap();

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("not found") || stderr.contains("does not exist"));
}

use escargot::CargoBuild;
use tempfile::TempDir;

#[test]
fn test_rename_basic() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let vex_bin = CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap();

    vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(&["save", "old-name", "qemu-system-x86_64", "-m", "2G"])
        .output()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(&["rename", "old-name", "new-name"])
        .output()
        .unwrap();

    assert!(output.status.success());

    assert!(!config_dir.join("old-name.json").exists());

    assert!(config_dir.join("new-name.json").exists());

    let config = std::fs::read_to_string(config_dir.join("new-name.json")).unwrap();
    assert!(config.contains("2G"));
}

#[test]
fn test_rename_nonexistent() {
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
        .args(&["rename", "nonexistent", "new-name"])
        .output()
        .unwrap();

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("not found") || stderr.contains("does not exist"));
}

#[test]
fn test_rename_preserves_description() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let vex_bin = CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap();

    vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(&[
            "save",
            "original",
            "-d",
            "Important configuration",
            "qemu-system-x86_64",
            "-m",
            "2G",
        ])
        .output()
        .unwrap();

    vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(&["rename", "original", "renamed"])
        .output()
        .unwrap();

    let config = std::fs::read_to_string(config_dir.join("renamed.json")).unwrap();
    assert!(config.contains("Important configuration"));
    assert!(config.contains("2G"));
}

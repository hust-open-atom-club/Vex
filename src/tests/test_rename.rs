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
        .args(["save", "old-name", "qemu-system-x86_64", "-m", "2G"])
        .output()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["rename", "old-name", "new-name"])
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
        .args(["rename", "nonexistent", "new-name"])
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
        .args([
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
        .args(["rename", "original", "renamed"])
        .output()
        .unwrap();

    let config = std::fs::read_to_string(config_dir.join("renamed.json")).unwrap();
    assert!(config.contains("Important configuration"));
    assert!(config.contains("2G"));
}

#[test]
fn test_rename_target_exists_with_force() {
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
        .args(["save", "source", "qemu-system-x86_64", "-m", "1G"])
        .output()
        .unwrap();

    vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["save", "target", "qemu-system-x86_64", "-m", "2G"])
        .output()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["rename", "-f", "source", "target"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!config_dir.join("source.json").exists());
    let content = std::fs::read_to_string(config_dir.join("target.json")).unwrap();
    assert!(content.contains("1G"));
}

#[test]
fn test_rename_with_desc_update() {
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
        .args(["save", "desc-vm", "qemu-system-x86_64"])
        .output()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["rename", "-d", "Updated description", "desc-vm", "new-desc"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let content = std::fs::read_to_string(config_dir.join("new-desc.json")).unwrap();
    assert!(content.contains("Updated description"));
}

#[test]
fn test_rename_invalid_new_name_rejected() {
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
        .args(["save", "valid-vm", "qemu-system-x86_64"])
        .output()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["rename", "valid-vm", "../bad-name"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(config_dir.join("valid-vm.json").exists());
}

#[test]
fn test_rename_preserves_qemu_version() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let config_json = r#"{
        "qemu_bin": "qemu-system-x86_64",
        "args": ["-m", "2G"],
        "desc": "test VM",
        "qemu_version": "9.0.1"
    }"#;
    std::fs::write(config_dir.join("version-vm.json"), config_json).unwrap();

    let vex_bin = CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["rename", "version-vm", "renamed-version"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let content = std::fs::read_to_string(config_dir.join("renamed-version.json")).unwrap();
    assert!(content.contains("9.0.1"));
}

#[test]
fn test_rename_then_print_shows_new_name() {
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
        .args([
            "save",
            "before-rename",
            "-d",
            "Rename test",
            "qemu-system-x86_64",
            "-m",
            "1G",
        ])
        .output()
        .unwrap();

    vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["rename", "before-rename", "after-rename"])
        .output()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["print", "after-rename"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("after-rename"));
    assert!(stdout.contains("1G"));
    assert!(stdout.contains("Rename test"));

    let old_output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["print", "before-rename"])
        .output()
        .unwrap();
    assert!(!old_output.status.success());
}

#[test]
fn test_rename_preserves_args() {
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
        .args([
            "save",
            "src-vm",
            "qemu-system-x86_64",
            "-m",
            "4G",
            "-smp",
            "8",
        ])
        .output()
        .unwrap();

    vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["rename", "src-vm", "dst-vm"])
        .output()
        .unwrap();

    let content = std::fs::read_to_string(config_dir.join("dst-vm.json")).unwrap();
    assert!(content.contains("4G"));
    assert!(content.contains("8"));
    assert!(content.contains("qemu-system-x86_64"));
}

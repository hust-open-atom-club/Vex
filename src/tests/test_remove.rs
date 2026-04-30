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
        .args(["rm", "nonexistent"])
        .output()
        .unwrap();

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("not found") || stderr.contains("does not exist"));
}

#[test]
fn test_remove_successful_delete() {
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
        .args(["save", "to-delete", "qemu-system-x86_64"])
        .output()
        .unwrap();

    assert!(config_dir.join("to-delete.json").exists());

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["rm", "to-delete"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!config_dir.join("to-delete.json").exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("deleted"));
}

#[test]
fn test_remove_invalid_name_rejected() {
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
        .args(["rm", "../escape"])
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn test_remove_then_list_empty() {
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
        .args(["save", "temp-vm", "qemu-system-x86_64"])
        .output()
        .unwrap();

    vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["rm", "temp-vm"])
        .output()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .arg("list")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
    assert!(stdout.contains("no configurations"));
}

#[test]
fn test_remove_twice_fails_second_time() {
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
        .args(["save", "once-vm", "qemu-system-x86_64"])
        .output()
        .unwrap();

    let first = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["rm", "once-vm"])
        .output()
        .unwrap();
    assert!(first.status.success());

    let second = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["rm", "once-vm"])
        .output()
        .unwrap();
    assert!(!second.status.success());
}

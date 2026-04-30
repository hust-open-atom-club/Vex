use escargot::CargoBuild;
use tempfile::TempDir;

fn vex_bin() -> escargot::CargoRun {
    CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap()
}

fn setup_config_dir() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();
    temp_dir
}

fn save_config(temp_dir: &TempDir, name: &str) {
    let config_dir = temp_dir.path().join(".vex");
    let vex = vex_bin();
    vex.command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["save", name, "/bin/true", "-nographic"])
        .output()
        .unwrap();
}

#[test]
fn exec_config_not_found() {
    let temp_dir = setup_config_dir();
    let config_dir = temp_dir.path().join(".vex");
    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["exec", "nonexistent"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("not found"));
}

#[test]
fn exec_debug_flag() {
    let temp_dir = setup_config_dir();
    save_config(&temp_dir, "debug-vm");
    let config_dir = temp_dir.path().join(".vex");
    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["exec", "-d", "debug-vm"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("DEBUG") || stdout.contains("GDB"));
}

#[test]
fn exec_full_flag() {
    let temp_dir = setup_config_dir();
    save_config(&temp_dir, "full-vm");
    let config_dir = temp_dir.path().join(".vex");
    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["exec", "-f", "full-vm"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("/bin/true") || stdout.contains("Args:"));
}

#[test]
fn exec_combined_debug_and_full_flags() {
    let temp_dir = setup_config_dir();
    save_config(&temp_dir, "combo-vm");
    let config_dir = temp_dir.path().join(".vex");
    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["exec", "-d", "-f", "combo-vm"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("DEBUG") || stdout.contains("GDB"));
    assert!(stdout.contains("/bin/true") || stdout.contains("Args:"));
}

#[test]
fn exec_invalid_name_rejected() {
    let temp_dir = setup_config_dir();
    let config_dir = temp_dir.path().join(".vex");
    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["exec", ".."])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn exec_nonexistent_binary_fails() {
    let temp_dir = setup_config_dir();
    let config_dir = temp_dir.path().join(".vex");

    let config_json = r#"{
        "qemu_bin": "/nonexistent/qemu-system-xyz",
        "args": ["-m", "2G"],
        "desc": null,
        "qemu_version": null
    }"#;
    std::fs::write(config_dir.join("bad-bin.json"), config_json).unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["exec", "bad-bin"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("launch") || stderr.contains("not found") || stderr.contains("qemu"));
}

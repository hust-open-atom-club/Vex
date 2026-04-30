use escargot::CargoBuild;
use tempfile::TempDir;

#[test]
fn test_list_empty() {
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
        .arg("list")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
    assert!(stdout.contains("no configurations"));
}

#[test]
fn test_list_single_config() {
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
        .args(["save", "test-vm", "qemu-system-x86_64", "-m", "2G"])
        .output()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .arg("list")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test-vm"));
}

#[test]
fn test_list_multiple_configs() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let vex_bin = CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap();

    let configs = vec![
        ("vm1", "First VM", "qemu-system-x86_64"),
        ("vm2", "Second VM", "qemu-system-arm"),
        ("vm3", "Third VM", "qemu-system-riscv64"),
    ];

    for (name, desc, qemu_bin) in &configs {
        vex_bin
            .command()
            .env("VEX_CONFIG_DIR", &config_dir)
            .args(["save", name, "-d", desc, qemu_bin])
            .output()
            .unwrap();
    }

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .arg("list")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vm1"));
    assert!(stdout.contains("vm2"));
    assert!(stdout.contains("vm3"));
    assert!(stdout.contains("First VM") || stdout.contains("qemu-system-x86_64"));
}

#[test]
fn test_list_shows_descriptions() {
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
            "documented-vm",
            "-d",
            "This is a well-documented VM",
            "qemu-system-x86_64",
        ])
        .output()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .arg("list")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("documented-vm"));
    assert!(stdout.contains("well-documented"));
}

#[test]
fn test_list_ignores_non_json_files() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    std::fs::write(config_dir.join("readme.txt"), "not a config").unwrap();
    std::fs::write(config_dir.join(".hidden"), "hidden file").unwrap();

    let vex_bin = CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap();

    vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["save", "real-vm", "qemu-system-x86_64"])
        .output()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .arg("list")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("real-vm"));
    assert!(!stdout.contains("readme"));
    assert!(!stdout.contains(".hidden"));
}

#[test]
fn test_list_ignores_invalid_json_files() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    std::fs::write(config_dir.join("bad.json"), "not valid json").unwrap();

    let vex_bin = CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap();

    vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["save", "good-vm", "qemu-system-x86_64"])
        .output()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .arg("list")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("good-vm"));
    assert!(!stdout.contains("bad"));
}

#[test]
fn test_list_no_description_shows_placeholder() {
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
        .args(["save", "nodesc-vm", "qemu-system-x86_64"])
        .output()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .arg("list")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("nodesc-vm"));
    assert!(stdout.contains("no description"));
}

#[test]
fn test_list_shows_qemu_binary_info() {
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
        .args(["save", "arm-vm", "qemu-system-aarch64", "-m", "2G"])
        .output()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .arg("list")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("qemu-system-aarch64"));
}

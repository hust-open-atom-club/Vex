use escargot::CargoBuild;
use tempfile::TempDir;

fn vex_bin() -> escargot::CargoRun {
    CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap()
}

#[test]
fn print_config_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["print", "nonexistent"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("not found"));
}

#[test]
fn print_full_output_format() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let vex = vex_bin();

    vex.command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args([
            "save",
            "print-test",
            "-d",
            "A test VM",
            "qemu-system-x86_64",
            "-m",
            "4G",
        ])
        .output()
        .unwrap();

    let output = vex
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["print", "print-test"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Configuration: print-test"));
    assert!(stdout.contains("A test VM"));
    assert!(stdout.contains("qemu-system-x86_64"));
    assert!(stdout.contains("4G"));
    assert!(stdout.contains("Full Command:"));
}

#[test]
fn print_shows_sorted_resources_and_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();
    let sha256 = "a".repeat(64);
    let config = serde_json::json!({
        "qemu_bin": "qemu-system-x86_64",
        "args": ["-m", "2G"],
        "resources": {
            "z_disk": {
                "path": "/images/disk.qcow2",
                "kind": "image",
                "sha256": sha256,
                "size": 4096,
                "url": "https://example.com/disk.qcow2"
            },
            "a_firmware": {
                "path": "/firmware/uefi.bin",
                "kind": "firmware"
            }
        }
    });
    std::fs::write(
        config_dir.join("resources.json"),
        serde_json::to_string_pretty(&config).unwrap(),
    )
    .unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["print", "resources"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Resources:"));
    assert!(stdout.contains("a_firmware  [Firmware]"));
    assert!(stdout.contains("path:   /firmware/uefi.bin"));
    assert!(stdout.contains("sha256: (none)"));
    assert!(stdout.contains("size:   (unknown)"));
    assert!(stdout.contains("url:    (none)"));
    assert!(stdout.contains("z_disk  [Image]"));
    assert!(stdout.contains("path:   /images/disk.qcow2"));
    assert!(stdout.contains(&format!("sha256: {}", "a".repeat(64))));
    assert!(stdout.contains("size:   4096 bytes"));
    assert!(stdout.contains("url:    https://example.com/disk.qcow2"));
    assert!(stdout.find("a_firmware").unwrap() < stdout.find("z_disk").unwrap());
}

#[test]
fn print_shows_empty_resources_state() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();
    let config = serde_json::json!({
        "qemu_bin": "qemu-system-x86_64",
        "args": []
    });
    std::fs::write(
        config_dir.join("empty.json"),
        serde_json::to_string_pretty(&config).unwrap(),
    )
    .unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["print", "empty"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Resources:\n  (none)"));
}

#[test]
fn print_no_description() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let vex = vex_bin();

    vex.command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["save", "nodesc", "qemu-system-x86_64", "-m", "2G"])
        .output()
        .unwrap();

    let output = vex
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["print", "nodesc"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Configuration: nodesc"));
    assert!(!stdout.contains("Description:"));
}

#[test]
fn print_no_args_shows_placeholder() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let vex = vex_bin();

    vex.command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["save", "noargs", "qemu-system-x86_64"])
        .output()
        .unwrap();

    let output = vex
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["print", "noargs"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("no arguments"));
}

#[test]
fn print_shows_numbered_args() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let vex = vex_bin();

    vex.command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args([
            "save",
            "numbered",
            "qemu-system-x86_64",
            "-m",
            "2G",
            "-smp",
            "4",
        ])
        .output()
        .unwrap();

    let output = vex
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["print", "numbered"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[0] -m"));
    assert!(stdout.contains("[1] 2G"));
    assert!(stdout.contains("[2] -smp"));
    assert!(stdout.contains("[3] 4"));
}

#[test]
fn print_shows_config_file_path() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let vex = vex_bin();

    vex.command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["save", "pathcheck", "qemu-system-x86_64"])
        .output()
        .unwrap();

    let output = vex
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["print", "pathcheck"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Configuration File:"));
    assert!(stdout.contains("pathcheck.json"));
}

#[test]
fn print_invalid_name_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["print", ".."])
        .output()
        .unwrap();

    assert!(!output.status.success());
}

use escargot::CargoBuild;
use tempfile::TempDir;

#[test]
fn test_save_basic_config() {
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
        .args(["save", "my-vm", "qemu-system-x86_64", "-m", "2G"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("my-vm") || stdout.contains("saved"));

    let config_file = config_dir.join("my-vm.json");
    assert!(config_file.exists());
}

#[test]
fn test_save_with_description() {
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
        .args([
            "save",
            "ubuntu-dev",
            "-d",
            "Ubuntu development VM",
            "qemu-system-x86_64",
            "-m",
            "4G",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());

    let config_file = config_dir.join("ubuntu-dev.json");
    let config_content = std::fs::read_to_string(config_file).unwrap();
    assert!(config_content.contains("Ubuntu development VM"));
    assert!(config_content.contains("4G"));
}

#[test]
fn test_save_missing_qemu_binary() {
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
        .args(["save", "test-vm"])
        .output()
        .unwrap();

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("required") || stderr.contains("missing") || stderr.contains("qemu"));
}

#[test]
fn test_save_complex_arguments() {
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
        .args([
            "save",
            "complex-vm",
            "qemu-system-x86_64",
            "-m",
            "8G",
            "-smp",
            "cores=4,threads=2",
            "-drive",
            "file=/path/to/disk.qcow2,format=qcow2",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());

    let config_file = config_dir.join("complex-vm.json");
    let config_content = std::fs::read_to_string(config_file).unwrap();
    assert!(config_content.contains("8G"));
    assert!(config_content.contains("cores=4,threads=2"));
    assert!(config_content.contains("disk.qcow2"));
}

#[test]
fn test_save_multiple_configs() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let vex_bin = CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap();

    for name in &["vm1", "vm2", "vm3"] {
        vex_bin
            .command()
            .env("VEX_CONFIG_DIR", &config_dir)
            .args(["save", name, "qemu-system-x86_64"])
            .output()
            .unwrap();
    }

    assert!(config_dir.join("vm1.json").exists());
    assert!(config_dir.join("vm2.json").exists());
    assert!(config_dir.join("vm3.json").exists());
}

#[test]
fn test_save_force_overwrite() {
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
        .args(["save", "overwrite-vm", "qemu-system-x86_64", "-m", "1G"])
        .output()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args([
            "save",
            "-f",
            "overwrite-vm",
            "qemu-system-x86_64",
            "-m",
            "4G",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let content = std::fs::read_to_string(config_dir.join("overwrite-vm.json")).unwrap();
    assert!(content.contains("4G"));
}

#[test]
fn test_save_invalid_name_path_traversal() {
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
        .args(["save", "../escape", "qemu-system-x86_64"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("validation") || stderr.contains("separator") || stderr.contains("name")
    );
}

#[test]
fn test_save_then_list_shows_config() {
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
            "listed-vm",
            "-d",
            "Should appear in list",
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

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("listed-vm"));
    assert!(stdout.contains("Should appear in list"));
}

#[test]
fn test_save_no_args_creates_valid_config() {
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
        .args(["save", "minimal-vm", "qemu-system-x86_64"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let content = std::fs::read_to_string(config_dir.join("minimal-vm.json")).unwrap();
    let config: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(config["qemu_bin"], "qemu-system-x86_64");
    assert!(config["args"].as_array().unwrap().is_empty());
}

#[test]
fn test_save_name_with_dots_and_hyphens() {
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
        .args(["save", "my-vm_v2.0", "qemu-system-x86_64"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(config_dir.join("my-vm_v2.0.json").exists());
}

#[test]
fn test_save_preserves_all_args() {
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
        .args([
            "save",
            "args-vm",
            "qemu-system-x86_64",
            "-m",
            "4G",
            "-smp",
            "cores=4,threads=2",
            "-drive",
            "file=disk.qcow2,format=qcow2",
            "-netdev",
            "user,id=net0",
            "-device",
            "virtio-net,netdev=net0",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let content = std::fs::read_to_string(config_dir.join("args-vm.json")).unwrap();
    let config: serde_json::Value = serde_json::from_str(&content).unwrap();
    let args = config["args"].as_array().unwrap();
    assert_eq!(args.len(), 10);
}

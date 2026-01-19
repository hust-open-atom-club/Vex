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
        .args(&["save", "my-vm", "qemu-system-x86_64", "-m", "2G"])
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
        .args(&[
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
        .args(&["save", "test-vm"])
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
        .args(&[
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
            .args(&["save", name, "qemu-system-x86_64"])
            .output()
            .unwrap();
    }

    assert!(config_dir.join("vm1.json").exists());
    assert!(config_dir.join("vm2.json").exists());
    assert!(config_dir.join("vm3.json").exists());
}

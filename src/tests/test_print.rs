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

    let output = vex.command()
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

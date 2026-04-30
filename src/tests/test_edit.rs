use escargot::CargoBuild;
use tempfile::TempDir;

#[test]
fn test_edit_nonexistent() {
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
        .args(["edit", "nonexistent"])
        .output()
        .unwrap();

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("not found")
            || stderr.contains("does not exist")
            || stderr.contains("cannot edit")
    );
}

#[cfg(unix)]
#[test]
fn test_edit_no_changes() {
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
        .env("EDITOR", "true")
        .args(["edit", "test-vm"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No changes made"));
}

#[cfg(unix)]
#[test]
fn test_edit_failing_editor() {
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
        .args(["save", "edit-fail", "qemu-system-x86_64", "-m", "2G"])
        .output()
        .unwrap();

    let output = vex_bin
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .env("EDITOR", "false")
        .args(["edit", "edit-fail"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("editor") || stderr.contains("failed"));
}

#[test]
fn test_edit_invalid_name_rejected() {
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
        .args(["edit", "../escape"])
        .output()
        .unwrap();

    assert!(!output.status.success());
}

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
    vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["save", name, "/bin/true", "-nographic"])
        .output()
        .unwrap();
}

#[test]
fn cli_help_shows_usage() {
    let output = vex_bin().command().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("vex"));
    assert!(stdout.contains("exec"));
    assert!(stdout.contains("save"));
    assert!(stdout.contains("list"));
    assert!(stdout.contains("print"));
    assert!(stdout.contains("rm"));
    assert!(stdout.contains("pull"));
    assert!(stdout.contains("push"));
}

#[test]
fn cli_help_subcommand_shows_usage() {
    let output = vex_bin()
        .command()
        .args(["exec", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("vex"));
    assert!(stdout.contains("exec"));
}

#[test]
fn cli_version_output() {
    let output = vex_bin().command().arg("--version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vex"));
    assert!(stdout.contains("0.4.0"));
}

#[test]
fn cli_no_subcommand_fails() {
    let output = vex_bin().command().output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("require") || stderr.contains("usage") || stderr.contains("subcommand")
    );
}

#[test]
fn cli_invalid_subcommand_fails() {
    let output = vex_bin()
        .command()
        .arg("not-a-real-command")
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn exec_default_output_hides_details() {
    let temp_dir = setup_config_dir();
    save_config(&temp_dir, "quiet-vm");
    let config_dir = temp_dir.path().join(".vex");
    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["exec", "quiet-vm"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Starting configuration"));
    assert!(
        !stdout.contains("QEMU:"),
        "default exec should not show QEMU path"
    );
    assert!(
        !stdout.contains("Args:"),
        "default exec should not show Args"
    );
    assert!(stdout.contains("quiet-vm"));
}

#[test]
fn exec_full_flag_shows_details() {
    let temp_dir = setup_config_dir();
    save_config(&temp_dir, "full-vm");
    let config_dir = temp_dir.path().join(".vex");
    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["exec", "-f", "full-vm"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Starting configuration"));
    assert!(stdout.contains("QEMU:"));
    assert!(stdout.contains("Args:"));
    assert!(stdout.contains("/bin/true"));
    assert!(stdout.contains("-nographic"));
}

#[test]
fn exec_long_full_flag_shows_details() {
    let temp_dir = setup_config_dir();
    save_config(&temp_dir, "long-vm");
    let config_dir = temp_dir.path().join(".vex");
    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["exec", "--full", "long-vm"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("QEMU:"));
    assert!(stdout.contains("Args:"));
}

#[test]
fn exec_default_vs_full_output_differ() {
    let temp_dir = setup_config_dir();
    save_config(&temp_dir, "diff-vm");
    let config_dir = temp_dir.path().join(".vex");

    let default_output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["exec", "diff-vm"])
        .output()
        .unwrap();
    assert!(default_output.status.success());
    let default_stdout = String::from_utf8_lossy(&default_output.stdout);

    let full_output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["exec", "-f", "diff-vm"])
        .output()
        .unwrap();
    assert!(full_output.status.success());
    let full_stdout = String::from_utf8_lossy(&full_output.stdout);

    assert_ne!(
        default_stdout, full_stdout,
        "default and --full output should differ"
    );
    assert!(
        full_stdout.len() > default_stdout.len(),
        "--full output should be longer than default"
    );
}

#[test]
fn save_then_exec_without_flag_does_not_leak_path() {
    let temp_dir = setup_config_dir();
    let config_dir = temp_dir.path().join(".vex");

    vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["save", "secure-vm", "/bin/true", "-m", "512"])
        .output()
        .unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["exec", "secure-vm"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("secure-vm"));
    assert!(
        !stdout.contains("/bin/true"),
        "default exec should not expose binary path"
    );
    assert!(
        !stdout.contains("512"),
        "default exec should not expose args"
    );
}

#[test]
fn exec_with_description_shows_in_header() {
    let temp_dir = setup_config_dir();
    let config_dir = temp_dir.path().join(".vex");

    vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["save", "desc-vm", "-d", "My development VM", "/bin/true"])
        .output()
        .unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["exec", "desc-vm"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("desc-vm"));
    assert!(stdout.contains("My development VM"));
}

#[test]
fn exec_without_description_omits_from_header() {
    let temp_dir = setup_config_dir();
    save_config(&temp_dir, "nodesc-vm");
    let config_dir = temp_dir.path().join(".vex");

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["exec", "nodesc-vm"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("nodesc-vm"));
    assert!(
        !stdout.contains("()"),
        "no description should not show empty parens"
    );
}

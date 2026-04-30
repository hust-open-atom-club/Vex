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
fn pull_invalid_remote_ref() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .env("VEX_REMOTE_URL", "https://example.com/repo.git")
        .args(["pull", "invalid-no-slash"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("invalid remote spec") || stderr.contains("must be in the form"));
}

#[test]
fn push_local_config_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .env("VEX_REMOTE_URL", "https://example.com/repo.git")
        .args(["push", "org/name:v1", "nonexistent-local"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("not found"));
}

#[test]
fn push_invalid_remote_ref() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .env("VEX_REMOTE_URL", "https://example.com/repo.git")
        .args(["push", "noslash", "some-config"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("invalid remote spec") || stderr.contains("must be in the form"));
}

#[test]
fn pull_without_remote_url_fails() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .env_remove("VEX_REMOTE_URL")
        .args(["pull", "org/config"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("remote")
            || stderr.contains("not configured")
            || stderr.contains("vex_remote_url")
    );
}

#[test]
fn push_without_remote_url_fails() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["save", "local-cfg", "qemu-system-x86_64"])
        .output()
        .unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .env_remove("VEX_REMOTE_URL")
        .args(["push", "org/remote:v1", "local-cfg"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("remote") || stderr.contains("not configured"));
}

#[test]
fn pull_path_traversal_in_remote_ref_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .env("VEX_REMOTE_URL", "https://example.com/repo.git")
        .args(["pull", "../evil/config"])
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn push_path_traversal_in_remote_ref_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["save", "cfg", "qemu-system-x86_64"])
        .output()
        .unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .env("VEX_REMOTE_URL", "https://example.com/repo.git")
        .args(["push", "../evil/config", "cfg"])
        .output()
        .unwrap();

    assert!(!output.status.success());
}

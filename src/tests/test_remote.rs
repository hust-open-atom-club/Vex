use escargot::CargoBuild;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn git(args: &[&str], cwd: &Path) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_remote_registry() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let remote_repo = temp_dir.path().join("remote.git");

    let output = Command::new("git")
        .args(["init", "--bare", remote_repo.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git init --bare failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    (temp_dir, remote_repo)
}

#[test]
fn test_push_publishes_versioned_and_latest_files() {
    let (_remote_guard, remote_repo) = create_remote_registry();
    let config_root = TempDir::new().unwrap();
    let config_dir = config_root.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let vex = CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap();

    let save_output = vex
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["save", "local-vm", "qemu-system-x86_64", "-m", "2G"])
        .output()
        .unwrap();
    assert!(save_output.status.success());

    let push_output = vex
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .env("VEX_REMOTE_URL", &remote_repo)
        .env("VEX_REMOTE_BRANCH", "main")
        .args(["push", "team/demo:v1", "local-vm"])
        .output()
        .unwrap();
    assert!(
        push_output.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push_output.stderr)
    );

    let inspect_root = TempDir::new().unwrap();
    git(
        &[
            "clone",
            remote_repo.to_str().unwrap(),
            inspect_root.path().join("repo").to_str().unwrap(),
        ],
        inspect_root.path(),
    );

    let repo_path = inspect_root.path().join("repo");
    git(&["checkout", "main"], &repo_path);

    let versioned = repo_path
        .join("configs")
        .join("team")
        .join("demo")
        .join("v1.json");
    let latest = repo_path
        .join("configs")
        .join("team")
        .join("demo")
        .join("latest.json");

    assert!(versioned.exists());
    assert!(latest.exists());

    let versioned_content = std::fs::read_to_string(versioned).unwrap();
    assert!(versioned_content.contains("\"tag\": \"v1\""));
    assert!(versioned_content.contains("\"qemu_bin\": \"qemu-system-x86_64\""));

    let latest_content = std::fs::read_to_string(latest).unwrap();
    assert!(latest_content.contains("\"tag\": \"v1\""));
}

#[test]
fn test_pull_restores_local_configuration_from_remote() {
    let (_remote_guard, remote_repo) = create_remote_registry();
    let source_root = TempDir::new().unwrap();
    let source_config_dir = source_root.path().join(".vex-source");
    std::fs::create_dir_all(&source_config_dir).unwrap();

    let vex = CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap();

    let save_output = vex
        .command()
        .env("VEX_CONFIG_DIR", &source_config_dir)
        .args([
            "save",
            "dev-box",
            "-d",
            "Development VM",
            "qemu-system-arm",
            "-m",
            "1G",
        ])
        .output()
        .unwrap();
    assert!(save_output.status.success());

    let push_output = vex
        .command()
        .env("VEX_CONFIG_DIR", &source_config_dir)
        .env("VEX_REMOTE_URL", &remote_repo)
        .env("VEX_REMOTE_BRANCH", "main")
        .args(["push", "team/dev-box:v2", "dev-box"])
        .output()
        .unwrap();
    assert!(push_output.status.success());

    let target_root = TempDir::new().unwrap();
    let target_config_dir = target_root.path().join(".vex-target");
    std::fs::create_dir_all(&target_config_dir).unwrap();

    let pull_output = vex
        .command()
        .env("VEX_CONFIG_DIR", &target_config_dir)
        .env("VEX_REMOTE_URL", &remote_repo)
        .env("VEX_REMOTE_BRANCH", "main")
        .args(["pull", "team/dev-box"])
        .output()
        .unwrap();
    assert!(
        pull_output.status.success(),
        "pull failed: {}",
        String::from_utf8_lossy(&pull_output.stderr)
    );

    let pulled_config = target_config_dir.join("dev-box.json");
    assert!(pulled_config.exists());

    let pulled_content = std::fs::read_to_string(pulled_config).unwrap();
    assert!(pulled_content.contains("\"qemu_bin\": \"qemu-system-arm\""));
    assert!(pulled_content.contains("\"Development VM\""));
    assert!(pulled_content.contains("\"1G\""));
}

#[test]
fn test_pull_force_overwrites_existing_local_configuration() {
    let (_remote_guard, remote_repo) = create_remote_registry();
    let source_root = TempDir::new().unwrap();
    let source_config_dir = source_root.path().join(".vex-source");
    std::fs::create_dir_all(&source_config_dir).unwrap();

    let vex = CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap();

    let save_output = vex
        .command()
        .env("VEX_CONFIG_DIR", &source_config_dir)
        .args([
            "save",
            "dev-box",
            "-d",
            "Remote VM",
            "qemu-system-aarch64",
            "-m",
            "4G",
        ])
        .output()
        .unwrap();
    assert!(save_output.status.success());

    let push_output = vex
        .command()
        .env("VEX_CONFIG_DIR", &source_config_dir)
        .env("VEX_REMOTE_URL", &remote_repo)
        .env("VEX_REMOTE_BRANCH", "main")
        .args(["push", "team/dev-box:v3", "dev-box"])
        .output()
        .unwrap();
    assert!(
        push_output.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push_output.stderr)
    );

    let target_root = TempDir::new().unwrap();
    let target_config_dir = target_root.path().join(".vex-target");
    std::fs::create_dir_all(&target_config_dir).unwrap();

    let local_save = vex
        .command()
        .env("VEX_CONFIG_DIR", &target_config_dir)
        .args([
            "save",
            "dev-box",
            "-d",
            "Old local VM",
            "qemu-system-x86_64",
            "-m",
            "2G",
        ])
        .output()
        .unwrap();
    assert!(local_save.status.success());

    let local_config = target_config_dir.join("dev-box.json");
    assert!(local_config.exists());
    let before = std::fs::read_to_string(&local_config).unwrap();
    assert!(before.contains("qemu-system-x86_64"));

    let pull_output = vex
        .command()
        .env("VEX_CONFIG_DIR", &target_config_dir)
        .env("VEX_REMOTE_URL", &remote_repo)
        .env("VEX_REMOTE_BRANCH", "main")
        .args(["pull", "-f", "team/dev-box:v3"])
        .output()
        .unwrap();
    assert!(
        pull_output.status.success(),
        "pull -f failed: {}",
        String::from_utf8_lossy(&pull_output.stderr)
    );

    let after = std::fs::read_to_string(&local_config).unwrap();
    assert!(
        after.contains("qemu-system-aarch64"),
        "local config should contain remote binary after force pull"
    );
    assert!(
        after.contains("4G"),
        "local config should contain remote memory arg after force pull"
    );
    assert!(
        !after.contains("qemu-system-x86_64"),
        "local config should no longer contain old binary after force pull"
    );
    assert!(
        !after.contains("Old local VM"),
        "local config should no longer contain old description after force pull"
    );
}

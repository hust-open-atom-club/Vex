use escargot::CargoBuild;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use tempfile::TempDir;

fn vex_bin() -> escargot::CargoRun {
    CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap()
}

fn save_empty_config(config_dir: &Path, name: &str) {
    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", config_dir)
        .args(["save", name, "qemu-system-x86_64"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "save '{}' failed: {}",
        name,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_resource_add_with_existing_file_records_checksum() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    save_empty_config(&config_dir, "cfg");

    let img = temp_dir.path().join("disk.img");
    std::fs::write(&img, b"hello").unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args([
            "resource",
            "add",
            "cfg",
            "disk",
            img.to_str().unwrap(),
            "--kind",
            "image",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "resource add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = std::fs::read_to_string(config_dir.join("cfg.json")).unwrap();
    let cfg: serde_json::Value = serde_json::from_str(&content).unwrap();
    let disk = &cfg["resources"]["disk"];
    assert_eq!(disk["kind"], "image");
    assert_eq!(disk["path"], img.to_str().unwrap());
    let sha = disk["sha256"].as_str().unwrap();
    assert_eq!(sha.len(), 64);
}

#[test]
fn test_resource_add_missing_file_without_allow_missing_fails() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    save_empty_config(&config_dir, "cfg");

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args([
            "resource",
            "add",
            "cfg",
            "disk",
            "/no/such/file/zzzzz",
            "--kind",
            "image",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("not found"),
        "expected stderr to contain 'not found', got: {}",
        stderr
    );
}

#[test]
fn test_resource_add_with_allow_missing_succeeds_without_checksum() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    save_empty_config(&config_dir, "cfg");

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args([
            "resource",
            "add",
            "cfg",
            "disk",
            "/no/such/file/zzzzz",
            "--kind",
            "image",
            "--allow-missing",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "resource add --allow-missing failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = std::fs::read_to_string(config_dir.join("cfg.json")).unwrap();
    let cfg: serde_json::Value = serde_json::from_str(&content).unwrap();
    let disk = &cfg["resources"]["disk"];
    assert!(disk.get("sha256").is_none() || disk["sha256"].is_null());
    assert!(disk.get("size").is_none() || disk["size"].is_null());
}

#[test]
fn test_resource_add_existing_key_without_force_prompts_and_aborts_on_no() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    save_empty_config(&config_dir, "cfg");

    let img1 = temp_dir.path().join("first.img");
    std::fs::write(&img1, b"first").unwrap();

    let first = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args([
            "resource",
            "add",
            "cfg",
            "disk",
            img1.to_str().unwrap(),
            "--kind",
            "image",
        ])
        .output()
        .unwrap();
    assert!(first.status.success());

    let img2 = temp_dir.path().join("second.img");
    std::fs::write(&img2, b"second").unwrap();

    let mut child = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args([
            "resource",
            "add",
            "cfg",
            "disk",
            img2.to_str().unwrap(),
            "--kind",
            "image",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"n\n").unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
    assert!(
        stdout.contains("cancelled"),
        "expected stdout to contain 'cancelled', got: {}",
        stdout
    );

    let content = std::fs::read_to_string(config_dir.join("cfg.json")).unwrap();
    let cfg: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(cfg["resources"]["disk"]["path"], img1.to_str().unwrap());
}

#[test]
fn test_resource_rm_removes_existing_key() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    save_empty_config(&config_dir, "cfg");

    let img = temp_dir.path().join("disk.img");
    std::fs::write(&img, b"x").unwrap();

    vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args([
            "resource",
            "add",
            "cfg",
            "disk",
            img.to_str().unwrap(),
            "--kind",
            "image",
        ])
        .output()
        .unwrap();

    let rm_output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["resource", "rm", "cfg", "disk"])
        .output()
        .unwrap();
    assert!(rm_output.status.success());

    let list_output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["resource", "list", "cfg"])
        .output()
        .unwrap();
    assert!(list_output.status.success());
    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(!stdout.contains("disk  ["));
}

#[test]
fn test_resource_list_prints_each_resource() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    save_empty_config(&config_dir, "cfg");

    let disk = temp_dir.path().join("d.img");
    std::fs::write(&disk, b"d").unwrap();
    let bios = temp_dir.path().join("bios.bin");
    std::fs::write(&bios, b"b").unwrap();

    for (key, kind, path) in [
        ("disk", "image", disk.to_str().unwrap()),
        ("bios", "firmware", bios.to_str().unwrap()),
    ] {
        let out = vex_bin()
            .command()
            .env("VEX_CONFIG_DIR", &config_dir)
            .args(["resource", "add", "cfg", key, path, "--kind", kind])
            .output()
            .unwrap();
        assert!(out.status.success());
    }

    let list_output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .args(["resource", "list", "cfg"])
        .output()
        .unwrap();
    assert!(list_output.status.success());
    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(stdout.contains("disk"));
    assert!(stdout.contains("bios"));
    assert!(stdout.contains(disk.to_str().unwrap()));
    assert!(stdout.contains(bios.to_str().unwrap()));
    assert!(stdout.contains("Image"));
    assert!(stdout.contains("Firmware"));
}

use escargot::CargoBuild;
use std::collections::HashMap;
use std::path::Path;
use tempfile::TempDir;

use crate::utils::hash::sha256_hex_of_bytes;

fn vex_bin() -> escargot::CargoRun {
    CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap()
}

fn write_cache_object(cache_root: &Path, content: &[u8]) -> String {
    let hash = sha256_hex_of_bytes(content);
    let dir = cache_root.join(&hash[..2]);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(&hash[2..]), content).unwrap();
    hash
}

fn write_config_referencing_hash(config_dir: &Path, name: &str, hash: &str) {
    let cfg = serde_json::json!({
        "qemu_bin": "qemu-system-x86_64",
        "args": [],
        "resources": {
            "disk": {
                "path": "/var/x.img",
                "kind": "image",
                "sha256": hash
            }
        }
    });
    std::fs::write(config_dir.join(format!("{}.json", name)), cfg.to_string()).unwrap();
}

fn setup_dirs() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let temp = TempDir::new().unwrap();
    let cfg = temp.path().join(".vex");
    let cache = temp.path().join(".vex-cache");
    std::fs::create_dir_all(&cfg).unwrap();
    std::fs::create_dir_all(&cache).unwrap();
    (temp, cfg, cache)
}

#[test]
fn test_cache_list_empty() {
    let (_g, cfg, cache) = setup_dirs();
    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_RESOURCE_CACHE_DIR", &cache)
        .args(["cache", "list"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Total: 0 objects"), "got: {}", stdout);
}

#[test]
fn test_cache_list_shows_object_with_zero_references() {
    let (_g, cfg, cache) = setup_dirs();
    let hash = write_cache_object(&cache, b"alpha");

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_RESOURCE_CACHE_DIR", &cache)
        .args(["cache", "list"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(&hash), "stdout missing hash: {}", stdout);
    assert!(
        stdout.contains("referenced: 0"),
        "stdout missing zero refs: {}",
        stdout
    );
}

#[test]
fn test_cache_list_shows_object_with_references() {
    let (_g, cfg, cache) = setup_dirs();
    let hash = write_cache_object(&cache, b"beta");
    write_config_referencing_hash(&cfg, "vm1", &hash);

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_RESOURCE_CACHE_DIR", &cache)
        .args(["cache", "list"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(&hash));
    assert!(stdout.contains("vm1"), "stdout missing vm1: {}", stdout);
    assert!(stdout.contains("referenced: 1"));
}

#[test]
fn test_cache_info_with_full_hash() {
    let (_g, cfg, cache) = setup_dirs();
    let hash = write_cache_object(&cache, b"gamma");

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_RESOURCE_CACHE_DIR", &cache)
        .args(["cache", "info", &hash])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(&hash));
    assert!(stdout.contains("size:"));
    assert!(stdout.contains("path:"));
}

#[test]
fn test_cache_info_with_unique_prefix() {
    let (_g, cfg, cache) = setup_dirs();
    let hash = write_cache_object(&cache, b"delta-unique");
    let prefix = &hash[..8];

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_RESOURCE_CACHE_DIR", &cache)
        .args(["cache", "info", prefix])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "cache info failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(&hash));
}

#[test]
fn test_cache_info_with_ambiguous_prefix_errors() {
    let (_g, cfg, cache) = setup_dirs();

    // Brute-force find two distinct payloads whose sha256 share a 4-hex-digit prefix.
    let mut seen: HashMap<String, Vec<u8>> = HashMap::new();
    let mut pair: Option<(Vec<u8>, Vec<u8>, String)> = None;
    for n in 0u64..200_000 {
        let bytes = format!("collision-{}", n).into_bytes();
        let h = sha256_hex_of_bytes(&bytes);
        let prefix = h[..4].to_string();
        if let Some(prev) = seen.get(&prefix) {
            pair = Some((prev.clone(), bytes, prefix));
            break;
        }
        seen.insert(prefix, bytes);
    }
    let (a, b, prefix) = pair.expect("could not find 4-hex prefix collision in 200k tries");
    write_cache_object(&cache, &a);
    write_cache_object(&cache, &b);

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_RESOURCE_CACHE_DIR", &cache)
        .args(["cache", "info", &prefix])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
    assert!(
        stderr.contains("ambiguous") || stderr.contains("multiple"),
        "expected stderr to flag ambiguity, got: {}",
        stderr
    );
}

#[test]
fn test_cache_rm_referenced_without_force_fails() {
    let (_g, cfg, cache) = setup_dirs();
    let hash = write_cache_object(&cache, b"epsilon");
    write_config_referencing_hash(&cfg, "vm1", &hash);

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_RESOURCE_CACHE_DIR", &cache)
        .args(["cache", "rm", &hash])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
    assert!(
        stderr.contains("referenced"),
        "stderr missing 'referenced': {}",
        stderr
    );

    let cache_path = cache.join(&hash[..2]).join(&hash[2..]);
    assert!(cache_path.exists(), "object should not have been deleted");
}

#[test]
fn test_cache_rm_referenced_with_force_succeeds() {
    let (_g, cfg, cache) = setup_dirs();
    let hash = write_cache_object(&cache, b"zeta");
    write_config_referencing_hash(&cfg, "vm1", &hash);

    let cache_path = cache.join(&hash[..2]).join(&hash[2..]);
    assert!(cache_path.exists());

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_RESOURCE_CACHE_DIR", &cache)
        .args(["cache", "rm", "--force", &hash])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(!cache_path.exists());
}

#[test]
fn test_cache_prune_dry_run_does_not_delete() {
    let (_g, cfg, cache) = setup_dirs();
    let referenced_hash = write_cache_object(&cache, b"eta-referenced");
    let unreferenced_hash = write_cache_object(&cache, b"theta-orphan");
    write_config_referencing_hash(&cfg, "vm1", &referenced_hash);

    let ref_path = cache
        .join(&referenced_hash[..2])
        .join(&referenced_hash[2..]);
    let orphan_path = cache
        .join(&unreferenced_hash[..2])
        .join(&unreferenced_hash[2..]);

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_RESOURCE_CACHE_DIR", &cache)
        .args(["cache", "prune", "--dry-run"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Would remove"), "got: {}", stdout);
    assert!(stdout.contains(&unreferenced_hash));

    assert!(ref_path.exists());
    assert!(orphan_path.exists());
}

#[test]
fn test_cache_prune_actually_deletes_unreferenced() {
    let (_g, cfg, cache) = setup_dirs();
    let referenced_hash = write_cache_object(&cache, b"iota-referenced");
    let unreferenced_hash = write_cache_object(&cache, b"kappa-orphan");
    write_config_referencing_hash(&cfg, "vm1", &referenced_hash);

    let ref_path = cache
        .join(&referenced_hash[..2])
        .join(&referenced_hash[2..]);
    let orphan_path = cache
        .join(&unreferenced_hash[..2])
        .join(&unreferenced_hash[2..]);

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_RESOURCE_CACHE_DIR", &cache)
        .args(["cache", "prune"])
        .output()
        .unwrap();
    assert!(out.status.success());

    assert!(ref_path.exists(), "referenced object must remain");
    assert!(
        !orphan_path.exists(),
        "unreferenced object should be deleted"
    );
}

#[test]
fn test_cache_respects_vex_resource_cache_dir() {
    let temp = TempDir::new().unwrap();
    let cfg = temp.path().join(".vex");
    std::fs::create_dir_all(&cfg).unwrap();

    // 1. With explicit VEX_RESOURCE_CACHE_DIR
    let custom_cache = temp.path().join("custom-cache");
    std::fs::create_dir_all(&custom_cache).unwrap();
    let hash_custom = write_cache_object(&custom_cache, b"in-custom-cache");

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_RESOURCE_CACHE_DIR", &custom_cache)
        .args(["cache", "list"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(&hash_custom));

    // 2. Without VEX_RESOURCE_CACHE_DIR: fallback is <config_dir parent>/resources
    let fallback_cache = temp.path().join("resources");
    std::fs::create_dir_all(&fallback_cache).unwrap();
    let hash_fallback = write_cache_object(&fallback_cache, b"in-fallback-cache");

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env_remove("VEX_RESOURCE_CACHE_DIR")
        .args(["cache", "list"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(&hash_fallback),
        "fallback cache miss: {}",
        stdout
    );
    // The custom cache hash should NOT be visible when env var is unset
    assert!(!stdout.contains(&hash_custom));
}

use escargot::CargoBuild;
use std::io::{self, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
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

fn create_bare_remote() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let remote = temp_dir.path().join("remote.git");
    let out = Command::new("git")
        .args(["init", "--bare", remote.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    (temp_dir, remote)
}

fn git_run(args: &[&str], cwd: &Path) {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write_published_to_remote(remote: &Path, id: &str, name: &str, tag: &str, payload: &str) {
    let temp = TempDir::new().unwrap();
    let work = temp.path().join("repo");
    git_run(
        &["clone", remote.to_str().unwrap(), work.to_str().unwrap()],
        temp.path(),
    );
    git_run(&["checkout", "-B", "main"], &work);
    let dest = work.join("configs").join(id).join(name);
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join(format!("{}.json", tag)), payload).unwrap();
    git_run(&["config", "user.name", "test"], &work);
    git_run(&["config", "user.email", "t@t"], &work);
    git_run(&["add", "."], &work);
    git_run(&["commit", "-m", "fixture"], &work);
    git_run(&["push", "origin", "main"], &work);
}

fn read_published_from_remote(remote: &Path, id: &str, name: &str, tag: &str) -> String {
    let temp = TempDir::new().unwrap();
    let work = temp.path().join("repo");
    git_run(
        &["clone", remote.to_str().unwrap(), work.to_str().unwrap()],
        temp.path(),
    );
    git_run(&["checkout", "main"], &work);
    let path = work
        .join("configs")
        .join(id)
        .join(name)
        .join(format!("{}.json", tag));
    std::fs::read_to_string(path).unwrap()
}

fn start_http_server(payload: Vec<u8>) -> (u16, Arc<AtomicBool>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    thread::spawn(move || {
        listener.set_nonblocking(true).ok();
        while !stop_clone.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut sock, _)) => {
                    sock.set_read_timeout(Some(Duration::from_secs(2))).ok();
                    sock.set_write_timeout(Some(Duration::from_secs(2))).ok();
                    let mut buf = [0u8; 4096];
                    let _ = sock.read(&mut buf);
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        payload.len()
                    );
                    let _ = sock.write_all(header.as_bytes());
                    let _ = sock.write_all(&payload);
                    let _ = sock.shutdown(std::net::Shutdown::Both);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    });
    (port, stop)
}

fn sha256_hex(bytes: &[u8]) -> String {
    crate::utils::hash::sha256_hex_of_bytes(bytes)
}

#[test]
fn test_push_with_resources_carrying_url_succeeds() {
    let (_remote_guard, remote_repo) = create_bare_remote();
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let cfg = serde_json::json!({
        "qemu_bin": "qemu-system-x86_64",
        "args": ["${res:disk}"],
        "resources": {
            "disk": {
                "path": "/var/lib/x.img",
                "kind": "image",
                "sha256": "a".repeat(64),
                "url": "https://example.com/x.img"
            }
        }
    });
    std::fs::write(config_dir.join("vm.json"), cfg.to_string()).unwrap();

    let push = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .env("VEX_REMOTE_URL", &remote_repo)
        .env("VEX_REMOTE_BRANCH", "main")
        .args(["push", "team/demo:v1", "vm"])
        .output()
        .unwrap();
    assert!(
        push.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );

    let content = read_published_from_remote(&remote_repo, "team", "demo", "v1");
    assert!(content.contains("\"schema_version\": 2"));
    assert!(content.contains("\"url\": \"https://example.com/x.img\""));
    assert!(content.contains("\"kind\": \"image\""));
}

#[test]
fn test_push_rejects_resource_without_url_or_sha256() {
    let (_remote_guard, remote_repo) = create_bare_remote();
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".vex");
    std::fs::create_dir_all(&config_dir).unwrap();

    let cfg = serde_json::json!({
        "qemu_bin": "qemu-system-x86_64",
        "args": [],
        "resources": {
            "disk": {
                "path": "/x",
                "kind": "image"
            }
        }
    });
    std::fs::write(config_dir.join("bad-vm.json"), cfg.to_string()).unwrap();

    let output = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &config_dir)
        .env("VEX_REMOTE_URL", &remote_repo)
        .env("VEX_REMOTE_BRANCH", "main")
        .args(["push", "team/bad:v1", "bad-vm"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("cannot be published"),
        "expected stderr to contain 'cannot be published', got: {}",
        stderr
    );
}

#[test]
fn test_pull_v1_remote_is_compatible() {
    let (_remote_guard, remote_repo) = create_bare_remote();

    let v1_payload = serde_json::json!({
        "schema_version": 1,
        "id": "team",
        "name": "legacy",
        "tag": "latest",
        "config": {
            "qemu_bin": "qemu-system-x86_64",
            "args": ["-m", "1G"]
        }
    })
    .to_string();
    write_published_to_remote(&remote_repo, "team", "legacy", "latest", &v1_payload);

    let target = TempDir::new().unwrap();
    let target_dir = target.path().join(".vex-target");
    std::fs::create_dir_all(&target_dir).unwrap();

    let pull = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &target_dir)
        .env("VEX_REMOTE_URL", &remote_repo)
        .env("VEX_REMOTE_BRANCH", "main")
        .args(["pull", "team/legacy"])
        .output()
        .unwrap();
    assert!(
        pull.status.success(),
        "pull failed: {}",
        String::from_utf8_lossy(&pull.stderr)
    );

    let local = std::fs::read_to_string(target_dir.join("legacy.json")).unwrap();
    let cfg: serde_json::Value = serde_json::from_str(&local).unwrap();
    assert_eq!(cfg["qemu_bin"], "qemu-system-x86_64");
    assert!(cfg.get("resources").is_none() || cfg["resources"].as_object().unwrap().is_empty());

    let list = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &target_dir)
        .args(["list"])
        .output()
        .unwrap();
    assert!(list.status.success());
    let out = String::from_utf8_lossy(&list.stdout);
    assert!(out.contains("legacy"));
}

#[test]
fn test_pull_with_fetch_resources_downloads_to_cache() {
    let (_remote_guard, remote_repo) = create_bare_remote();
    let payload = b"hello world";
    let sha = sha256_hex(payload);
    let (port, stop) = start_http_server(payload.to_vec());

    let v2_payload = serde_json::json!({
        "schema_version": 2,
        "id": "team",
        "name": "withres",
        "tag": "latest",
        "config": {
            "qemu_bin": "qemu-system-x86_64",
            "args": ["${res:disk}"],
            "resources": {
                "disk": {
                    "path": "/placeholder",
                    "kind": "image",
                    "sha256": sha,
                    "url": format!("http://127.0.0.1:{}/x.img", port)
                }
            }
        }
    })
    .to_string();
    write_published_to_remote(&remote_repo, "team", "withres", "latest", &v2_payload);

    let target = TempDir::new().unwrap();
    let target_dir = target.path().join(".vex-target");
    std::fs::create_dir_all(&target_dir).unwrap();
    let cache = target.path().join("cache");

    let pull = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &target_dir)
        .env("VEX_REMOTE_URL", &remote_repo)
        .env("VEX_REMOTE_BRANCH", "main")
        .args([
            "pull",
            "--fetch-resources",
            "--resource-dir",
            cache.to_str().unwrap(),
            "team/withres",
        ])
        .output()
        .unwrap();
    assert!(
        pull.status.success(),
        "pull failed: stderr={}",
        String::from_utf8_lossy(&pull.stderr)
    );

    let prefix = &sha[..2];
    let rest = &sha[2..];
    let cache_path = cache.join(prefix).join(rest);
    assert!(
        cache_path.exists(),
        "cache file not found at {:?}",
        cache_path
    );
    assert_eq!(std::fs::read(&cache_path).unwrap(), payload);

    let local = std::fs::read_to_string(target_dir.join("withres.json")).unwrap();
    let cfg: serde_json::Value = serde_json::from_str(&local).unwrap();
    assert_eq!(
        cfg["resources"]["disk"]["path"],
        cache_path.to_str().unwrap()
    );

    stop.store(true, Ordering::Relaxed);
}

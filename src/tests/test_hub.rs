use escargot::CargoBuild;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

use crate::utils::hash::sha256_hex_of_bytes;

fn vex_bin() -> escargot::CargoRun {
    CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap()
}

fn handle_request<H>(mut sock: TcpStream, handler: Arc<H>)
where
    H: Fn(&str) -> (u16, Vec<u8>) + Send + Sync + 'static,
{
    sock.set_read_timeout(Some(Duration::from_secs(2))).ok();
    sock.set_write_timeout(Some(Duration::from_secs(2))).ok();
    let mut buf = [0u8; 4096];
    let mut total = 0;
    loop {
        let read_buf = &mut buf[total..];
        if read_buf.is_empty() {
            break;
        }
        match sock.read(read_buf) {
            Ok(0) => break,
            Ok(n) => {
                total += n;
                if buf[..total].windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let request = String::from_utf8_lossy(&buf[..total]);
    let path_full = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_string();
    let path = path_full
        .split_once('?')
        .map(|(p, _)| p.to_string())
        .unwrap_or(path_full);
    let (status, body) = handler(&path);
    let status_text = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "Error",
    };
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
        status,
        status_text,
        body.len()
    );
    let _ = sock.write_all(header.as_bytes());
    let _ = sock.write_all(&body);
    let _ = sock.shutdown(std::net::Shutdown::Both);
}

fn serve<H>(listener: TcpListener, handler: H) -> Arc<AtomicBool>
where
    H: Fn(&str) -> (u16, Vec<u8>) + Send + Sync + 'static,
{
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    let h = Arc::new(handler);
    thread::spawn(move || {
        listener.set_nonblocking(true).ok();
        while !stop_clone.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((sock, _)) => {
                    let h2 = h.clone();
                    thread::spawn(move || handle_request(sock, h2));
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    });
    stop
}

fn bind_listener() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{}/", port);
    (listener, url)
}

fn temp_config_dir() -> (TempDir, std::path::PathBuf) {
    let temp = TempDir::new().unwrap();
    let cfg = temp.path().join(".vex");
    std::fs::create_dir_all(&cfg).unwrap();
    (temp, cfg)
}

#[test]
fn test_hub_search_match() {
    let (listener, base_url) = bind_listener();
    let stop = serve(listener, |path| {
        if path == "/index.json" {
            let body = r#"{
                "schema_version": 1,
                "entries": [
                    {"id":"team","name":"demo-arm64","latest_tag":"v1","tags":["v1"],"summary":"ARM demo board","kind":"demo","updated_at":"2026-01-01T00:00:00Z"},
                    {"id":"lab","name":"x86","latest_tag":"latest","tags":["latest"],"summary":"x86 sandbox","kind":"board","updated_at":"2026-01-02T00:00:00Z"},
                    {"id":"fw","name":"uboot","latest_tag":"v1","tags":["v1"],"summary":"U-Boot images","kind":"firmware","updated_at":"2026-01-03T00:00:00Z"}
                ]
            }"#;
            (200, body.as_bytes().to_vec())
        } else {
            (404, b"".to_vec())
        }
    });
    let (_g, cfg) = temp_config_dir();
    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_HUB_URL", &base_url)
        .args(["hub", "search", "demo"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "search failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("team/demo-arm64"));
    assert!(!stdout.contains("lab/x86"));
    assert!(!stdout.contains("fw/uboot"));
    stop.store(true, Ordering::Relaxed);
}

#[test]
fn test_hub_search_no_match() {
    let (listener, base_url) = bind_listener();
    let stop = serve(listener, |path| {
        if path == "/index.json" {
            let body = r#"{"schema_version":1,"entries":[]}"#;
            (200, body.as_bytes().to_vec())
        } else {
            (404, b"".to_vec())
        }
    });
    let (_g, cfg) = temp_config_dir();
    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_HUB_URL", &base_url)
        .args(["hub", "search", "qqq"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("No entries found"));
    stop.store(true, Ordering::Relaxed);
}

fn published_v2_json(qemu_bin: &str) -> Vec<u8> {
    let cfg = serde_json::json!({
        "schema_version": 2,
        "id": "team",
        "name": "demo",
        "tag": "v1",
        "config": {
            "qemu_bin": qemu_bin,
            "args": ["-m", "1G"],
            "desc": "demo description"
        }
    });
    cfg.to_string().into_bytes()
}

#[test]
fn test_hub_info_existing() {
    let (listener, base_url) = bind_listener();
    let stop = serve(listener, |path| {
        if path == "/configs/team/demo/v1.json" {
            (200, published_v2_json("qemu-system-arm"))
        } else {
            (404, b"".to_vec())
        }
    });
    let (_g, cfg) = temp_config_dir();
    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_HUB_URL", &base_url)
        .args(["hub", "info", "team/demo:v1"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "info failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("qemu-system-arm"));
    assert!(stdout.contains("demo description"));
    assert!(stdout.contains("[0] -m"));
    stop.store(true, Ordering::Relaxed);
}

#[test]
fn test_hub_info_missing_returns_404_error() {
    let (listener, base_url) = bind_listener();
    let stop = serve(listener, |_path| (404, b"missing".to_vec()));
    let (_g, cfg) = temp_config_dir();
    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_HUB_URL", &base_url)
        .args(["hub", "info", "team/ghost:v1"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
    assert!(
        stderr.contains("not found"),
        "expected stderr to contain 'not found', got: {}",
        stderr
    );
    stop.store(true, Ordering::Relaxed);
}

#[test]
fn test_hub_install_writes_local_config() {
    let (listener, base_url) = bind_listener();
    let stop = serve(listener, |path| {
        if path == "/configs/team/demo/v1.json" {
            (200, published_v2_json("qemu-system-x86_64"))
        } else {
            (404, b"".to_vec())
        }
    });
    let (_g, cfg) = temp_config_dir();
    let cache = _g.path().join("cache");

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_HUB_URL", &base_url)
        .env("VEX_RESOURCE_CACHE_DIR", &cache)
        .args(["hub", "install", "team/demo:v1"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let local = std::fs::read_to_string(cfg.join("demo.json")).unwrap();
    assert!(local.contains("qemu-system-x86_64"));
    assert!(local.contains("demo description"));
    stop.store(true, Ordering::Relaxed);
}

#[test]
fn test_hub_install_with_as_renames() {
    let (listener, base_url) = bind_listener();
    let stop = serve(listener, |path| {
        if path == "/configs/team/demo/v1.json" {
            (200, published_v2_json("qemu-system-x86_64"))
        } else {
            (404, b"".to_vec())
        }
    });
    let (_g, cfg) = temp_config_dir();
    let cache = _g.path().join("cache");

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_HUB_URL", &base_url)
        .env("VEX_RESOURCE_CACHE_DIR", &cache)
        .args(["hub", "install", "team/demo:v1", "--as", "my-demo"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(cfg.join("my-demo.json").exists());
    assert!(!cfg.join("demo.json").exists());
    stop.store(true, Ordering::Relaxed);
}

#[test]
fn test_hub_install_with_fetch_resources_downloads() {
    let (listener, base_url) = bind_listener();

    let payload = b"hub-binary-payload".to_vec();
    let payload_for_handler = payload.clone();
    let sha = sha256_hex_of_bytes(&payload);
    let resource_url = format!("{}res/disk.img", base_url);

    let cfg_json = serde_json::json!({
        "schema_version": 2,
        "id": "team",
        "name": "demo",
        "tag": "v1",
        "config": {
            "qemu_bin": "qemu-system-x86_64",
            "args": ["${res:disk}"],
            "resources": {
                "disk": {
                    "path": "/placeholder",
                    "kind": "image",
                    "sha256": sha.clone(),
                    "url": resource_url
                }
            }
        }
    })
    .to_string()
    .into_bytes();

    let stop = serve(listener, move |path| {
        if path == "/configs/team/demo/v1.json" {
            (200, cfg_json.clone())
        } else if path == "/res/disk.img" {
            (200, payload_for_handler.clone())
        } else {
            (404, b"".to_vec())
        }
    });

    let (_g, cfg) = temp_config_dir();
    let cache = _g.path().join("cache");

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_HUB_URL", &base_url)
        .args([
            "hub",
            "install",
            "team/demo:v1",
            "--fetch-resources",
            "--resource-dir",
            cache.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "install failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let prefix = &sha[..2];
    let rest = &sha[2..];
    let cache_path = cache.join(prefix).join(rest);
    assert!(cache_path.exists(), "cache miss at {:?}", cache_path);
    assert_eq!(std::fs::read(&cache_path).unwrap(), payload);

    let local = std::fs::read_to_string(cfg.join("demo.json")).unwrap();
    let cfg_value: serde_json::Value = serde_json::from_str(&local).unwrap();
    assert_eq!(
        cfg_value["resources"]["disk"]["path"],
        cache_path.to_str().unwrap()
    );

    stop.store(true, Ordering::Relaxed);
}

#[test]
fn test_hub_list_filters_by_kind() {
    let (listener, base_url) = bind_listener();
    let stop = serve(listener, |path| {
        if path == "/index.json" {
            let body = r#"{
                "schema_version": 1,
                "entries": [
                    {"id":"a","name":"one","latest_tag":"v1","tags":[],"summary":"alpha","kind":"demo","updated_at":""},
                    {"id":"b","name":"two","latest_tag":"v1","tags":[],"summary":"beta","kind":"board","updated_at":""},
                    {"id":"c","name":"three","latest_tag":"v1","tags":[],"summary":"gamma","kind":"demo","updated_at":""}
                ]
            }"#;
            (200, body.as_bytes().to_vec())
        } else {
            (404, b"".to_vec())
        }
    });
    let (_g, cfg) = temp_config_dir();

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_HUB_URL", &base_url)
        .args(["hub", "list", "--kind", "demo"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("a/one"));
    assert!(stdout.contains("c/three"));
    assert!(!stdout.contains("b/two"));
    assert!(stdout.contains("Total: 2 entries"));
    stop.store(true, Ordering::Relaxed);
}

fn index_with_one_entry(latest_tag: &str) -> Vec<u8> {
    serde_json::json!({
        "schema_version": 1,
        "entries": [{
            "id": "team",
            "name": "demo",
            "latest_tag": latest_tag,
            "tags": [latest_tag],
            "summary": "demo summary",
            "kind": "demo",
            "updated_at": "2026-01-01T00:00:00Z"
        }]
    })
    .to_string()
    .into_bytes()
}

#[test]
fn test_hub_info_resolves_omitted_tag_via_index_latest_tag() {
    let (listener, base_url) = bind_listener();
    let stop = serve(listener, |path| {
        if path == "/index.json" {
            (200, index_with_one_entry("v2"))
        } else if path == "/configs/team/demo/v2.json" {
            (200, published_v2_json("qemu-system-resolved"))
        } else if path == "/configs/team/demo/latest.json" {
            // The client must NOT request this; we 404 it to fail loudly if it does.
            (404, b"latest.json should never be requested".to_vec())
        } else {
            (404, b"".to_vec())
        }
    });
    let (_g, cfg) = temp_config_dir();
    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_HUB_URL", &base_url)
        .args(["hub", "info", "team/demo"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "info failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("qemu-system-resolved"));
    assert!(stdout.contains(":v2"));
    stop.store(true, Ordering::Relaxed);
}

#[test]
fn test_hub_info_explicit_tag_does_not_query_index() {
    // When a tag is explicit, the client should never need /index.json.
    // We prove this by serving v3.json but 404-ing /index.json.
    let (listener, base_url) = bind_listener();
    let stop = serve(listener, |path| {
        if path == "/configs/team/demo/v3.json" {
            (200, published_v2_json("qemu-system-explicit"))
        } else {
            (404, b"".to_vec())
        }
    });
    let (_g, cfg) = temp_config_dir();
    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_HUB_URL", &base_url)
        .args(["hub", "info", "team/demo:v3"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "explicit-tag info failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("qemu-system-explicit"));
    stop.store(true, Ordering::Relaxed);
}

#[test]
fn test_hub_install_resolves_omitted_tag_via_index() {
    let (listener, base_url) = bind_listener();
    let stop = serve(listener, |path| {
        if path == "/index.json" {
            (200, index_with_one_entry("v2"))
        } else if path == "/configs/team/demo/v2.json" {
            (200, published_v2_json("qemu-system-installed"))
        } else {
            // Includes /configs/team/demo/latest.json — must not be queried.
            (404, b"".to_vec())
        }
    });
    let (_g, cfg) = temp_config_dir();
    let cache = _g.path().join("cache");

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_HUB_URL", &base_url)
        .env("VEX_RESOURCE_CACHE_DIR", &cache)
        .args(["hub", "install", "team/demo", "--as", "my-demo"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(cfg.join("my-demo.json").exists());
    assert!(!cfg.join("demo.json").exists());
    assert!(!cfg.join("latest.json").exists());

    let local = std::fs::read_to_string(cfg.join("my-demo.json")).unwrap();
    assert!(local.contains("qemu-system-installed"));
    stop.store(true, Ordering::Relaxed);
}

#[test]
fn test_hub_install_rejects_path_traversal_in_as() {
    let (listener, base_url) = bind_listener();
    let stop = serve(listener, |path| {
        if path == "/configs/team/demo/v1.json" {
            (200, published_v2_json("qemu-system-x86_64"))
        } else {
            (404, b"".to_vec())
        }
    });
    let (g, cfg) = temp_config_dir();

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_HUB_URL", &base_url)
        .args(["hub", "install", "team/demo:v1", "--as", "../evil"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
    assert!(
        stderr.contains("separator") || stderr.contains("name") || stderr.contains("validation"),
        "stderr did not flag a name validation issue: {}",
        stderr
    );

    // The traversal target — a sibling of cfg — must not exist.
    let parent = g.path();
    assert!(!parent.join("evil.json").exists());
    assert!(!cfg.join("../evil.json").exists());
    stop.store(true, Ordering::Relaxed);
}

#[cfg(unix)]
#[test]
fn test_hub_install_rejects_absolute_path_in_as() {
    let (listener, base_url) = bind_listener();
    let stop = serve(listener, |path| {
        if path == "/configs/team/demo/v1.json" {
            (200, published_v2_json("qemu-system-x86_64"))
        } else {
            (404, b"".to_vec())
        }
    });
    let (_g, cfg) = temp_config_dir();

    // Use a unique sentinel path under /tmp so we can prove nothing landed there.
    let sentinel = format!(
        "/tmp/vex-test-evil-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    // Pre-condition: target must not exist.
    let target = format!("{}.json", sentinel);
    let _ = std::fs::remove_file(&target);
    assert!(!std::path::Path::new(&target).exists());

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_HUB_URL", &base_url)
        .args(["hub", "install", "team/demo:v1", "--as", &sentinel])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
    assert!(
        stderr.contains("separator") || stderr.contains("name") || stderr.contains("validation"),
        "stderr did not flag a name validation issue: {}",
        stderr
    );

    assert!(!std::path::Path::new(&target).exists());
    stop.store(true, Ordering::Relaxed);
}

#[test]
fn test_hub_install_fetch_resources_backfills_sha256_when_remote_omits_it() {
    let (listener, base_url) = bind_listener();
    let payload = b"hub-omits-sha-payload".to_vec();
    let payload_for_handler = payload.clone();
    let expected_sha = sha256_hex_of_bytes(&payload);
    let resource_url = format!("{}res/disk.img", base_url);

    // Remote PublishedConfig publishes the resource by url only — no sha256.
    let cfg_json = serde_json::json!({
        "schema_version": 2,
        "id": "team",
        "name": "demo",
        "tag": "v1",
        "config": {
            "qemu_bin": "qemu-system-x86_64",
            "args": ["${res:disk}"],
            "resources": {
                "disk": {
                    "path": "/placeholder",
                    "kind": "image",
                    "url": resource_url
                }
            }
        }
    })
    .to_string()
    .into_bytes();

    let stop = serve(listener, move |path| {
        if path == "/configs/team/demo/v1.json" {
            (200, cfg_json.clone())
        } else if path == "/res/disk.img" {
            (200, payload_for_handler.clone())
        } else {
            (404, b"".to_vec())
        }
    });

    let (_g, cfg) = temp_config_dir();
    let cache = _g.path().join("cache");

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_HUB_URL", &base_url)
        .args([
            "hub",
            "install",
            "team/demo:v1",
            "--fetch-resources",
            "--resource-dir",
            cache.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let local = std::fs::read_to_string(cfg.join("demo.json")).unwrap();
    let cfg_value: serde_json::Value = serde_json::from_str(&local).unwrap();
    let disk = &cfg_value["resources"]["disk"];
    assert_eq!(
        disk["sha256"].as_str(),
        Some(expected_sha.as_str()),
        "sha256 was not backfilled: {:?}",
        disk
    );
    assert_eq!(
        disk["size"].as_u64(),
        Some(payload.len() as u64),
        "size was not backfilled: {:?}",
        disk
    );

    stop.store(true, Ordering::Relaxed);
}

#[test]
fn test_hub_list_does_not_prefix_v_to_tag() {
    let (listener, base_url) = bind_listener();
    let stop = serve(listener, |path| {
        if path == "/index.json" {
            let body = r#"{
                "schema_version": 1,
                "entries": [
                    {"id":"a","name":"one","latest_tag":"latest","tags":["latest"],"summary":"A","kind":"demo","updated_at":""},
                    {"id":"b","name":"two","latest_tag":"v1","tags":["v1"],"summary":"B","kind":"demo","updated_at":""},
                    {"id":"c","name":"three","latest_tag":"main","tags":["main"],"summary":"C","kind":"demo","updated_at":""}
                ]
            }"#;
            (200, body.as_bytes().to_vec())
        } else {
            (404, b"".to_vec())
        }
    });
    let (_g, cfg) = temp_config_dir();

    let out = vex_bin()
        .command()
        .env("VEX_CONFIG_DIR", &cfg)
        .env("VEX_HUB_URL", &base_url)
        .args(["hub", "list"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("latest"));
    assert!(stdout.contains("v1"));
    assert!(stdout.contains("main"));

    assert!(
        !stdout.contains("vlatest"),
        "tag should not be prefixed with 'v': {}",
        stdout
    );
    assert!(
        !stdout.contains("vv1"),
        "tag should not be prefixed with 'v': {}",
        stdout
    );
    assert!(
        !stdout.contains("vmain"),
        "tag should not be prefixed with 'v': {}",
        stdout
    );

    stop.store(true, Ordering::Relaxed);
}

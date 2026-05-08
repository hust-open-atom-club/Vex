use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::error::{VexError, VexResult};
use crate::utils::hash::sha256_hex_of_file;

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Cache layout: `<root>/<sha256[0..2]>/<sha256[2..]>`.
/// Exposed for the cache management commands so they can enumerate / locate objects.
pub fn cache_object_path(root: &Path, sha256_hex: &str) -> PathBuf {
    debug_assert_eq!(sha256_hex.len(), 64);
    let (prefix, suffix) = sha256_hex.split_at(2);
    root.join(prefix).join(suffix)
}

pub fn fetch_to_file(url: &str, dest: &Path) -> VexResult<()> {
    let scheme = url.split_once("://").map(|(s, _)| s).unwrap_or("");
    match scheme {
        "http" | "https" => {}
        _ => {
            return Err(VexError::UnsupportedResourceScheme {
                scheme: scheme.to_string(),
            });
        }
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| VexError::IoError {
            path: parent.to_path_buf(),
            operation: "create resource cache directory".into(),
            source: e,
        })?;
    }

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(30))
        .timeout_read(Duration::from_secs(30))
        .build();

    let response = agent
        .get(url)
        .call()
        .map_err(|e| VexError::ResourceFetchFailed {
            url: url.to_string(),
            reason: e.to_string(),
        })?;

    let mut reader = response.into_reader();
    let mut file = File::create(dest).map_err(|e| VexError::ResourceFetchFailed {
        url: url.to_string(),
        reason: format!("create destination file: {}", e),
    })?;
    io::copy(&mut reader, &mut file).map_err(|e| VexError::ResourceFetchFailed {
        url: url.to_string(),
        reason: format!("write response body: {}", e),
    })?;
    Ok(())
}

pub fn fetch_to_cache(
    url: &str,
    expected_sha256: Option<&str>,
    cache_dir: &Path,
) -> VexResult<PathBuf> {
    fs::create_dir_all(cache_dir).map_err(|e| VexError::IoError {
        path: cache_dir.to_path_buf(),
        operation: "create resource cache directory".into(),
        source: e,
    })?;

    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_name = format!(".tmp-{}-{}", std::process::id(), n);
    let tmp_path = cache_dir.join(&tmp_name);

    if let Err(e) = fetch_to_file(url, &tmp_path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }

    let actual = match sha256_hex_of_file(&tmp_path) {
        Ok(s) => s,
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            return Err(VexError::ResourceFetchFailed {
                url: url.to_string(),
                reason: format!("compute sha256 of downloaded file: {}", e),
            });
        }
    };

    if let Some(expected) = expected_sha256
        && actual != expected.to_lowercase()
    {
        let _ = fs::remove_file(&tmp_path);
        return Err(VexError::ResourceChecksumMismatch {
            key: url.to_string(),
            expected: expected.to_string(),
            actual,
        });
    }

    let final_path = cache_object_path(cache_dir, &actual);
    if let Some(parent) = final_path.parent() {
        fs::create_dir_all(parent).map_err(|e| VexError::IoError {
            path: parent.to_path_buf(),
            operation: "create cache subdirectory".into(),
            source: e,
        })?;
    }

    if let Err(rename_err) = fs::rename(&tmp_path, &final_path) {
        if let Err(copy_err) = fs::copy(&tmp_path, &final_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(VexError::IoError {
                path: final_path,
                operation: format!("rename failed ({}); copy fallback failed", rename_err),
                source: copy_err,
            });
        }
        let _ = fs::remove_file(&tmp_path);
    }

    Ok(final_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::hash::sha256_hex_of_bytes;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering as AtOrd};
    use std::thread;
    use tempfile::TempDir;

    fn start_server<F>(handler: F) -> (u16, Arc<AtomicBool>)
    where
        F: Fn(&mut std::net::TcpStream) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();
        thread::spawn(move || {
            listener.set_nonblocking(true).ok();
            while !stop_clone.load(AtOrd::Relaxed) {
                match listener.accept() {
                    Ok((mut sock, _)) => {
                        sock.set_read_timeout(Some(Duration::from_secs(2))).ok();
                        sock.set_write_timeout(Some(Duration::from_secs(2))).ok();
                        // drain request headers
                        let mut buf = [0u8; 4096];
                        let _ = sock.read(&mut buf);
                        handler(&mut sock);
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

    fn ok_handler(payload: &'static [u8]) -> impl Fn(&mut std::net::TcpStream) {
        move |sock| {
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                payload.len()
            );
            let _ = sock.write_all(header.as_bytes());
            let _ = sock.write_all(payload);
        }
    }

    #[test]
    fn fetch_to_cache_success_with_matching_sha() {
        let payload = b"hello world";
        let expected = sha256_hex_of_bytes(payload);
        let (port, stop) = start_server(ok_handler(payload));
        let url = format!("http://127.0.0.1:{}/data", port);
        let cache = TempDir::new().unwrap();

        let path = fetch_to_cache(&url, Some(&expected), cache.path()).unwrap();
        let content = fs::read(&path).unwrap();
        assert_eq!(content, payload);
        let prefix = &expected[..2];
        let rest = &expected[2..];
        assert_eq!(path, cache.path().join(prefix).join(rest));
        stop.store(true, AtOrd::Relaxed);
    }

    #[test]
    fn fetch_to_cache_checksum_mismatch_cleans_tmp() {
        let payload = b"hello world";
        let wrong = "0".repeat(64);
        let (port, stop) = start_server(ok_handler(payload));
        let url = format!("http://127.0.0.1:{}/data", port);
        let cache = TempDir::new().unwrap();

        let err = fetch_to_cache(&url, Some(&wrong), cache.path()).unwrap_err();
        match err {
            VexError::ResourceChecksumMismatch {
                expected, actual, ..
            } => {
                assert_eq!(expected, wrong);
                assert_eq!(actual, sha256_hex_of_bytes(payload));
            }
            other => panic!("expected ResourceChecksumMismatch, got {:?}", other),
        }

        let leftover: Vec<_> = fs::read_dir(cache.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(".tmp-"))
            .collect();
        assert!(leftover.is_empty(), "tmp file not cleaned: {:?}", leftover);
        stop.store(true, AtOrd::Relaxed);
    }

    #[test]
    fn fetch_to_cache_404_yields_fetch_failed() {
        let (port, stop) = start_server(|sock| {
            let _ = sock.write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        });
        let url = format!("http://127.0.0.1:{}/missing", port);
        let cache = TempDir::new().unwrap();

        let err = fetch_to_cache(&url, None, cache.path()).unwrap_err();
        match err {
            VexError::ResourceFetchFailed { .. } => {}
            other => panic!("expected ResourceFetchFailed, got {:?}", other),
        }
        stop.store(true, AtOrd::Relaxed);
    }

    #[test]
    fn fetch_to_cache_unsupported_scheme() {
        let cache = TempDir::new().unwrap();
        let err = fetch_to_cache("file:///etc/hosts", None, cache.path()).unwrap_err();
        match err {
            VexError::UnsupportedResourceScheme { scheme } => assert_eq!(scheme, "file"),
            other => panic!("expected UnsupportedResourceScheme, got {:?}", other),
        }
    }
}

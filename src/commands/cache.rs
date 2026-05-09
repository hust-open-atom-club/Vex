use clap::{Args, Subcommand};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{config_dir, parse_config_json, resource_cache_dir};
use crate::error::{VexError, VexResult};

#[derive(Args, Debug)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: CacheCommands,
}

#[derive(Subcommand, Debug)]
pub enum CacheCommands {
    /// List all cached resource objects
    List(CacheListArgs),
    /// Show details of a single cached object
    Info(CacheInfoArgs),
    /// Remove a single cached object by hash
    Rm(CacheRmArgs),
    /// Remove all unreferenced cached objects
    Prune(CachePruneArgs),
}

#[derive(Args, Debug)]
pub struct CacheListArgs;

#[derive(Args, Debug)]
pub struct CacheInfoArgs {
    /// Full sha256 hex (64 chars) or unique prefix (>=4 chars)
    pub hash: String,
}

#[derive(Args, Debug)]
pub struct CacheRmArgs {
    pub hash: String,
    /// Remove even if currently referenced by saved configurations
    #[arg(short = 'f', long = "force")]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct CachePruneArgs {
    /// Show what would be removed without deleting
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

#[derive(Debug)]
struct CacheObject {
    hash: String,
    path: PathBuf,
    size: u64,
}

fn enumerate_cache_objects(root: &Path) -> VexResult<Vec<CacheObject>> {
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    let outer_iter = fs::read_dir(root).map_err(|e| VexError::IoError {
        path: root.to_path_buf(),
        operation: "read cache root".into(),
        source: e,
    })?;
    for outer in outer_iter {
        let outer = outer.map_err(|e| VexError::IoError {
            path: root.to_path_buf(),
            operation: "read cache directory entry".into(),
            source: e,
        })?;
        let outer_path = outer.path();
        let outer_name = match outer.file_name().to_str() {
            Some(n) => n.to_string(),
            None => continue,
        };
        if outer_name.len() != 2 || !outer_name.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        if !outer_path.is_dir() {
            continue;
        }
        let inner_iter = match fs::read_dir(&outer_path) {
            Ok(it) => it,
            Err(_) => continue,
        };
        for inner in inner_iter {
            let inner = match inner {
                Ok(e) => e,
                Err(_) => continue,
            };
            let inner_name = match inner.file_name().to_str() {
                Some(n) => n.to_string(),
                None => continue,
            };
            if inner_name.starts_with('.') {
                continue;
            }
            if inner_name.len() != 62 || !inner_name.chars().all(|c| c.is_ascii_hexdigit()) {
                continue;
            }
            let inner_path = inner.path();
            let meta = match fs::metadata(&inner_path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if !meta.is_file() {
                continue;
            }
            let hash = format!("{}{}", outer_name, inner_name);
            out.push(CacheObject {
                hash,
                path: inner_path,
                size: meta.len(),
            });
        }
    }
    out.sort_by(|a, b| a.hash.cmp(&b.hash));
    Ok(out)
}

fn collect_referenced_hashes() -> VexResult<HashMap<String, Vec<String>>> {
    let dir = config_dir()?;
    // Best-effort: if the cache root cannot be resolved, we still scan by sha256.
    let cache_root = resource_cache_dir().ok();
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    if !dir.exists() {
        return Ok(map);
    }
    let entries = fs::read_dir(&dir).map_err(|e| VexError::IoError {
        path: dir.clone(),
        operation: "read config directory".into(),
        source: e,
    })?;
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let cfg = match parse_config_json(&content) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for r in cfg.resources.values() {
            // Primary path: explicit sha256.
            if let Some(sha) = &r.sha256 {
                map.entry(sha.to_lowercase())
                    .or_default()
                    .push(name.clone());
                continue;
            }
            // Fallback: recognize `path` pointing into the cache layout.
            // This catches resources that were fetched via url-only entries
            // before sha256 backfill landed, or that pre-date backfill.
            if let Some(root) = &cache_root
                && let Some(sha) = recover_hash_from_cache_path(&r.path, root)
            {
                map.entry(sha).or_default().push(name.clone());
            }
        }
    }
    for v in map.values_mut() {
        v.sort();
        v.dedup();
    }
    Ok(map)
}

/// Try to recover a sha256 hex from a cache-path-like value.
///
/// Returns `Some(hex)` only if the path lives under `cache_root` and the
/// last two components are the canonical `<2 hex>/<62 hex>` shard layout.
/// Does **not** canonicalize or follow symlinks (avoids IO + attack surface);
/// the file does not need to currently exist.
fn recover_hash_from_cache_path(path: &str, cache_root: &Path) -> Option<String> {
    let p = Path::new(path);
    if !p.starts_with(cache_root) {
        return None;
    }
    let mut rev = p.components().rev();
    let last = rev.next()?;
    let second_last = rev.next()?;
    let last_str = match last {
        std::path::Component::Normal(s) => s.to_str()?,
        _ => return None,
    };
    let second_str = match second_last {
        std::path::Component::Normal(s) => s.to_str()?,
        _ => return None,
    };
    if second_str.len() != 2 || !second_str.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    if last_str.len() != 62 || !last_str.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!(
        "{}{}",
        second_str.to_lowercase(),
        last_str.to_lowercase()
    ))
}

fn resolve_hash_prefix<'a>(prefix: &str, objects: &'a [CacheObject]) -> VexResult<&'a CacheObject> {
    if prefix.len() < 4 {
        return Err(VexError::ValidationError {
            field: Some("hash".into()),
            reason: "hash prefix must be at least 4 characters".into(),
        });
    }
    if !prefix.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(VexError::ValidationError {
            field: Some("hash".into()),
            reason: "hash must be hexadecimal".into(),
        });
    }
    let lower = prefix.to_lowercase();
    let matches: Vec<&CacheObject> = objects
        .iter()
        .filter(|o| o.hash.starts_with(&lower))
        .collect();
    match matches.len() {
        0 => Err(VexError::ValidationError {
            field: Some("hash".into()),
            reason: format!("no cached object matches prefix '{}'", prefix),
        }),
        1 => Ok(matches[0]),
        n => Err(VexError::ValidationError {
            field: Some("hash".into()),
            reason: format!("ambiguous prefix '{}' matches {} objects", prefix, n),
        }),
    }
}

fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;
    if bytes < KB {
        format!("{} bytes", bytes)
    } else if bytes < MB {
        format!("{} bytes (~{:.1} KB)", bytes, bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{} bytes (~{:.1} MB)", bytes, bytes as f64 / MB as f64)
    } else {
        format!("{} bytes (~{:.1} GB)", bytes, bytes as f64 / GB as f64)
    }
}

fn print_object(obj: &CacheObject, refs: &[String]) {
    println!("{}", obj.hash);
    println!("  path:       {}", obj.path.display());
    println!("  size:       {}", human_size(obj.size));
    if refs.is_empty() {
        println!("  referenced: 0 config(s)");
    } else {
        println!(
            "  referenced: {} config(s): {}",
            refs.len(),
            refs.join(", ")
        );
    }
}

pub fn cache_list_command(_args: CacheListArgs) -> VexResult<()> {
    let root = resource_cache_dir()?;
    let objects = enumerate_cache_objects(&root)?;
    let refs = collect_referenced_hashes()?;

    let mut total_bytes: u64 = 0;
    let mut unreferenced: u64 = 0;
    for obj in &objects {
        let r = refs.get(&obj.hash).cloned().unwrap_or_default();
        print_object(obj, &r);
        println!();
        total_bytes += obj.size;
        if r.is_empty() {
            unreferenced += 1;
        }
    }

    println!(
        "Total: {} objects, {} bytes total",
        objects.len(),
        total_bytes
    );
    if unreferenced > 0 {
        println!(
            "Unreferenced: {} object{} (use 'vex cache prune' to clean up)",
            unreferenced,
            if unreferenced == 1 { "" } else { "s" }
        );
    }
    Ok(())
}

pub fn cache_info_command(args: CacheInfoArgs) -> VexResult<()> {
    let root = resource_cache_dir()?;
    let objects = enumerate_cache_objects(&root)?;
    let obj = resolve_hash_prefix(&args.hash, &objects)?;
    let refs = collect_referenced_hashes()?;
    let r = refs.get(&obj.hash).cloned().unwrap_or_default();
    print_object(obj, &r);
    Ok(())
}

pub fn cache_rm_command(args: CacheRmArgs) -> VexResult<()> {
    let root = resource_cache_dir()?;
    let objects = enumerate_cache_objects(&root)?;
    let obj = resolve_hash_prefix(&args.hash, &objects)?;
    let refs = collect_referenced_hashes()?;
    let referenced = refs.get(&obj.hash).cloned().unwrap_or_default();

    if !referenced.is_empty() && !args.force {
        return Err(VexError::ValidationError {
            field: Some("hash".into()),
            reason: format!(
                "object is referenced by {} config(s); use --force to delete anyway",
                referenced.len()
            ),
        });
    }

    let bytes = obj.size;
    let path = obj.path.clone();
    let hash = obj.hash.clone();

    fs::remove_file(&path).map_err(|e| VexError::IoError {
        path: path.clone(),
        operation: "remove cache object".into(),
        source: e,
    })?;
    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir(parent);
    }

    println!("Removed {} ({})", hash, human_size(bytes));
    Ok(())
}

pub fn cache_prune_command(args: CachePruneArgs) -> VexResult<()> {
    let root = resource_cache_dir()?;
    let objects = enumerate_cache_objects(&root)?;
    let refs = collect_referenced_hashes()?;

    let mut total_bytes: u64 = 0;
    let mut count: u64 = 0;
    for obj in &objects {
        if refs.contains_key(&obj.hash) {
            continue;
        }
        if args.dry_run {
            println!("Would remove: {} ({})", obj.hash, human_size(obj.size));
        } else {
            if let Err(e) = fs::remove_file(&obj.path) {
                eprintln!("warning: failed to remove {}: {}", obj.path.display(), e);
                continue;
            }
            if let Some(parent) = obj.path.parent() {
                let _ = fs::remove_dir(parent);
            }
            println!("Removed {} ({})", obj.hash, human_size(obj.size));
        }
        total_bytes += obj.size;
        count += 1;
    }

    if args.dry_run {
        println!("{} objects ({} bytes) would be freed", count, total_bytes);
    } else {
        println!("{} objects ({} bytes) freed", count, total_bytes);
    }
    Ok(())
}

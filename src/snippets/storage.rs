use std::path::PathBuf;

use super::types::{Snippet, SnippetFile};
use crate::config::{config_dir, vex_root_dir};
use crate::error::{VexError, VexResult};

/// User snippets file at `<vex_root>/snippets.json`. Default mode:
/// `~/.vex/snippets.json` (sibling to `configs/`).
pub fn snippets_file() -> VexResult<PathBuf> {
    Ok(vex_root_dir()?.join("snippets.json"))
}

/// One-time migration of `~/.vex/configs/snippets.json` (0.4.1 GA) to
/// `~/.vex/snippets.json`. Default mode only — env mode is a no-op
/// because the two paths coincide.
fn migrate_legacy_snippets_path() -> VexResult<()> {
    let env_set = matches!(std::env::var("VEX_CONFIG_DIR"), Ok(v) if !v.is_empty());
    if env_set {
        return Ok(());
    }
    let new_path = snippets_file()?;
    let old_path = config_dir()?.join("snippets.json");
    if !new_path.exists() && old_path.exists() {
        std::fs::rename(&old_path, &new_path).map_err(|e| VexError::IoError {
            path: old_path,
            operation: "migrate snippets.json from configs/".to_string(),
            source: e,
        })?;
    }
    Ok(())
}

/// Load user snippets from disk.
///
/// Returns:
/// - `Ok(Vec::new())` when the file does not exist (fresh install).
/// - `Ok(snippets)` on successful load.
/// - `Err(VexError::ConfigParseFailed { .. })` when the file exists but
///   is corrupt JSON. (Re-uses the existing path-less variant — see
///   crate::error::VexError; no new variant is introduced.)
/// - `Err(VexError::IoError { .. })` on filesystem errors.
pub fn load_user_snippets() -> VexResult<Vec<Snippet>> {
    migrate_legacy_snippets_path()?;
    let path = snippets_file()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| VexError::IoError {
        path: path.clone(),
        operation: "read snippets.json".to_string(),
        source: e,
    })?;
    let file: SnippetFile =
        serde_json::from_str(&content).map_err(|e| VexError::ConfigParseFailed { source: e })?;
    Ok(file.snippets)
}

/// Load builtin + user snippets, merged. See `merge` for the merge contract.
///
/// User file errors (corrupt JSON, IO failure) propagate as `Err`. TUI
/// callers should catch these and fall back to builtin-only with a
/// user-facing message.
pub fn load_merged() -> VexResult<Vec<Snippet>> {
    let builtin = super::builtin::builtin_snippets();
    let user = load_user_snippets()?;
    Ok(merge(builtin, user))
}

/// Persist a slice of user snippets to `$VEX_CONFIG_DIR/snippets.json`.
/// Atomically replaces the file via write-to-tempfile + rename.
///
/// Callers MUST pass user-defined snippets only; builtin entries would
/// otherwise be saved as user entries and break the merge logic.
pub fn save_user_snippets(snippets: &[Snippet]) -> VexResult<()> {
    let path = snippets_file()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| VexError::IoError {
            path: parent.to_path_buf(),
            operation: "create snippets dir".to_string(),
            source: e,
        })?;
    }
    let file = SnippetFile {
        schema_version: SnippetFile::CURRENT_VERSION,
        snippets: snippets.to_vec(),
    };
    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| VexError::ConfigParseFailed { source: e })?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &json).map_err(|e| VexError::IoError {
        path: tmp.clone(),
        operation: "write snippets.json (tmp)".to_string(),
        source: e,
    })?;
    std::fs::rename(&tmp, &path).map_err(|e| VexError::IoError {
        path: path.clone(),
        operation: "rename snippets.json".to_string(),
        source: e,
    })?;
    Ok(())
}

/// Pure merge function — pub(crate) for unit testing.
///
/// Rules:
/// 1. Start from `builtin` in its original order.
/// 2. Each user snippet whose `name` matches a builtin replaces that
///    builtin in place (preserving position).
/// 3. User snippets whose `name` matches no builtin are appended at
///    the end, in their original file order.
pub(crate) fn merge(builtin: Vec<Snippet>, user: Vec<Snippet>) -> Vec<Snippet> {
    let mut result = builtin;
    for u in user {
        if let Some(slot) = result.iter_mut().find(|b| b.name == u.name) {
            *slot = u;
        } else {
            result.push(u);
        }
    }
    result
}

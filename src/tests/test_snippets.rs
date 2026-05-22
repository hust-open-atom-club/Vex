use crate::error::VexError;
use crate::snippets::storage::merge;
use crate::snippets::{
    Snippet, SnippetCategory, SnippetFile, builtin_snippets, load_merged, load_user_snippets,
};
use std::collections::HashSet;
use std::sync::Mutex;

/// Serializes tests that mutate the process-global `VEX_CONFIG_DIR` env
/// var. Independent from the one in `tests::test_tui`; cross-module
/// collisions are theoretically possible if both modules' env-tests race,
/// but in practice the per-test windows are short and tempdirs are unique
/// — flagged as a follow-up to extract a shared test-utils helper.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn set_config_dir(dir: &std::path::Path) {
    // SAFETY: callers hold ENV_LOCK; only the snippets/refresh tests touch
    // VEX_CONFIG_DIR in-process.
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir);
    }
}

// --- Builtin library checks -----------------------------------------------

#[test]
fn builtin_snippets_count_is_42() {
    assert_eq!(builtin_snippets().len(), 42);
}

#[test]
fn builtin_snippets_unique_names() {
    let names: Vec<String> = builtin_snippets().into_iter().map(|s| s.name).collect();
    let unique: HashSet<&String> = names.iter().collect();
    assert_eq!(
        unique.len(),
        names.len(),
        "duplicate snippet names in builtin library"
    );
}

#[test]
fn builtin_snippets_category_counts() {
    let all = builtin_snippets();
    let count = |c: SnippetCategory| all.iter().filter(|s| s.category == c).count();
    assert_eq!(count(SnippetCategory::Memory), 5);
    assert_eq!(count(SnippetCategory::Cpu), 6);
    assert_eq!(count(SnippetCategory::Machine), 4);
    assert_eq!(count(SnippetCategory::Storage), 7);
    assert_eq!(count(SnippetCategory::Network), 4);
    assert_eq!(count(SnippetCategory::Display), 5);
    assert_eq!(count(SnippetCategory::Debug), 5);
    assert_eq!(count(SnippetCategory::Kernel), 6);
}

#[test]
fn builtin_snippets_all_have_description() {
    for s in builtin_snippets() {
        let desc = s
            .description
            .as_ref()
            .unwrap_or_else(|| panic!("snippet {:?} has no description", s.name));
        assert!(
            !desc.trim().is_empty(),
            "snippet {:?} has empty description",
            s.name
        );
    }
}

// --- Load tests (in-process VEX_CONFIG_DIR) -------------------------------

#[test]
fn load_user_snippets_returns_empty_when_file_missing() {
    let _guard = lock_env();
    let dir = tempfile::tempdir().unwrap();
    set_config_dir(dir.path());
    let result = load_user_snippets().unwrap();
    assert!(result.is_empty());
}

#[test]
fn load_user_snippets_returns_err_on_corrupt_json() {
    let _guard = lock_env();
    let dir = tempfile::tempdir().unwrap();
    set_config_dir(dir.path());
    std::fs::write(dir.path().join("snippets.json"), "not valid json").unwrap();
    let err = load_user_snippets().unwrap_err();
    match err {
        VexError::ConfigParseFailed { .. } => {}
        other => panic!("expected ConfigParseFailed, got {:?}", other),
    }
}

#[test]
fn load_user_snippets_returns_parsed_when_valid() {
    let _guard = lock_env();
    let dir = tempfile::tempdir().unwrap();
    set_config_dir(dir.path());

    let file = SnippetFile {
        schema_version: SnippetFile::CURRENT_VERSION,
        snippets: vec![
            Snippet {
                name: "alpha".to_string(),
                args: vec!["-m".to_string(), "512M".to_string()],
                category: SnippetCategory::Memory,
                description: None,
            },
            Snippet {
                name: "beta".to_string(),
                args: vec!["-smp".to_string(), "2".to_string()],
                category: SnippetCategory::Cpu,
                description: Some("two cores".to_string()),
            },
        ],
    };
    let json = serde_json::to_string(&file).unwrap();
    std::fs::write(dir.path().join("snippets.json"), json).unwrap();

    let loaded = load_user_snippets().unwrap();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].name, "alpha");
    assert_eq!(loaded[1].name, "beta");
}

// --- Merge tests (pure, no IO) --------------------------------------------

fn user_snippet(name: &str, category: SnippetCategory, args: Vec<&str>) -> Snippet {
    Snippet {
        name: name.to_string(),
        args: args.into_iter().map(|s| s.to_string()).collect(),
        category,
        description: None,
    }
}

#[test]
fn merge_no_user_returns_only_builtin() {
    let merged = merge(builtin_snippets(), Vec::new());
    assert_eq!(merged.len(), 42);
}

#[test]
fn merge_user_override_replaces_builtin_in_place() {
    let user = vec![user_snippet(
        "2G memory",
        SnippetCategory::Memory,
        vec!["-m", "2048M"],
    )];
    let merged = merge(builtin_snippets(), user);
    assert_eq!(merged.len(), 42, "override must not change length");
    let two_g = merged
        .iter()
        .find(|s| s.name == "2G memory")
        .expect("override entry missing");
    assert_eq!(two_g.args, vec!["-m".to_string(), "2048M".to_string()]);
}

#[test]
fn merge_user_only_appended_at_end() {
    let user = vec![user_snippet(
        "my custom snippet",
        SnippetCategory::Debug,
        vec!["-trace", "events=on"],
    )];
    let merged = merge(builtin_snippets(), user);
    assert_eq!(merged.len(), 43);
    assert_eq!(merged.last().unwrap().name, "my custom snippet");
}

#[test]
fn merge_preserves_builtin_order() {
    let merged = merge(builtin_snippets(), Vec::new());
    let head: Vec<&str> = merged.iter().take(5).map(|s| s.name.as_str()).collect();
    assert_eq!(
        head,
        vec![
            "512M memory",
            "1G memory",
            "2G memory",
            "4G memory",
            "8G memory",
        ]
    );
}

#[test]
fn load_merged_integration() {
    let _guard = lock_env();
    let dir = tempfile::tempdir().unwrap();
    set_config_dir(dir.path());

    let file = SnippetFile {
        schema_version: SnippetFile::CURRENT_VERSION,
        snippets: vec![
            user_snippet("1G memory", SnippetCategory::Memory, vec!["-m", "1024M"]),
            user_snippet("my unique", SnippetCategory::Debug, vec!["-d", "all"]),
        ],
    };
    std::fs::write(
        dir.path().join("snippets.json"),
        serde_json::to_string(&file).unwrap(),
    )
    .unwrap();

    let merged = load_merged().unwrap();
    assert_eq!(merged.len(), 43, "42 builtins minus 1 override + 2 user");
    // Override applied:
    let one_g = merged.iter().find(|s| s.name == "1G memory").unwrap();
    assert_eq!(one_g.args, vec!["-m".to_string(), "1024M".to_string()]);
    // Unique appended at end:
    assert_eq!(merged.last().unwrap().name, "my unique");
}

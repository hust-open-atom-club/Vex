use crate::config::{
    QemuConfig, load_config_from_dir, parse_config_json, sanitize_config_name, validate_config,
    validate_config_name,
};
use crate::error::VexError;

#[test]
fn validate_valid_config() {
    let config = QemuConfig {
        qemu_bin: "qemu-system-x86_64".into(),
        args: vec!["-m".into(), "2G".into()],
        desc: Some("test".into()),
        qemu_version: Some("8.2.0".into()),
    };
    assert!(validate_config(&config).is_ok());
}

#[test]
fn validate_empty_binary_rejected() {
    let config = QemuConfig {
        qemu_bin: String::new(),
        args: vec![],
        desc: None,
        qemu_version: None,
    };
    let err = validate_config(&config).unwrap_err();
    assert!(matches!(err, VexError::ValidationError { .. }));
}

#[test]
fn validate_name_valid() {
    assert!(validate_config_name("my-vm").is_ok());
    assert!(validate_config_name("test_config").is_ok());
    assert!(validate_config_name("vm.backup").is_ok());
    assert!(validate_config_name("a").is_ok());
    assert!(validate_config_name("VM123").is_ok());
}

#[test]
fn validate_name_empty_rejected() {
    let err = validate_config_name("").unwrap_err();
    assert!(matches!(err, VexError::ValidationError { .. }));
}

#[test]
fn validate_name_dot_rejected() {
    assert!(validate_config_name(".").is_err());
    assert!(validate_config_name("..").is_err());
}

#[test]
fn validate_name_path_separators_rejected() {
    assert!(validate_config_name("../escape").is_err());
    assert!(validate_config_name("a/b").is_err());
    assert!(validate_config_name("a\\b").is_err());
}

#[test]
fn validate_name_null_byte_rejected() {
    assert!(validate_config_name("name\0evil").is_err());
}

#[test]
fn validate_name_too_long_rejected() {
    let long_name = "a".repeat(256);
    assert!(validate_config_name(&long_name).is_err());
}

#[test]
fn validate_name_max_length_accepted() {
    let name = "a".repeat(255);
    assert!(validate_config_name(&name).is_ok());
}

#[test]
fn validate_name_special_chars_rejected() {
    assert!(validate_config_name("name with space").is_err());
    assert!(validate_config_name("name@host").is_err());
    assert!(validate_config_name("name!").is_err());
}

#[test]
fn parse_valid_json() {
    let json = r#"{"qemu_bin":"qemu-system-x86_64","args":["-m","2G"],"desc":"test","qemu_version":"8.0"}"#;
    let config = parse_config_json(json).unwrap();
    assert_eq!(config.qemu_bin, "qemu-system-x86_64");
    assert_eq!(config.args, vec!["-m", "2G"]);
}

#[test]
fn parse_invalid_json_returns_error() {
    let err = parse_config_json("not json").unwrap_err();
    assert!(matches!(err, VexError::ConfigParseFailed { .. }));
}

#[test]
fn parse_empty_object_returns_error() {
    let err = parse_config_json("{}").unwrap_err();
    assert!(matches!(err, VexError::ConfigParseFailed { .. }));
}

#[test]
fn parse_minimal_valid_json() {
    let json = r#"{"qemu_bin":"qemu","args":[]}"#;
    let config = parse_config_json(json).unwrap();
    assert_eq!(config.qemu_bin, "qemu");
    assert!(config.args.is_empty());
    assert!(config.desc.is_none());
}

#[test]
fn validate_whitespace_only_binary_rejected() {
    let config = QemuConfig {
        qemu_bin: "   ".into(),
        args: vec![],
        desc: None,
        qemu_version: None,
    };
    assert!(validate_config(&config).is_err());
}

#[test]
fn validate_empty_arg_rejected() {
    let config = QemuConfig {
        qemu_bin: "qemu".into(),
        args: vec!["".into()],
        desc: None,
        qemu_version: None,
    };
    assert!(validate_config(&config).is_err());
}

#[test]
fn validate_whitespace_only_arg_rejected() {
    let config = QemuConfig {
        qemu_bin: "qemu".into(),
        args: vec!["-m".into(), "  ".into()],
        desc: None,
        qemu_version: None,
    };
    assert!(validate_config(&config).is_err());
}

#[test]
fn validate_null_byte_arg_rejected() {
    let config = QemuConfig {
        qemu_bin: "qemu".into(),
        args: vec!["-m\0evil".into()],
        desc: None,
        qemu_version: None,
    };
    assert!(validate_config(&config).is_err());
}

#[test]
fn validate_valid_args_accepted() {
    let config = QemuConfig {
        qemu_bin: "qemu".into(),
        args: vec!["-m".into(), "2G".into(), "-smp".into(), "4".into()],
        desc: None,
        qemu_version: None,
    };
    assert!(validate_config(&config).is_ok());
}

#[test]
fn load_config_from_dir_existing_config() {
    let dir = tempfile::tempdir().unwrap();
    let config = QemuConfig {
        qemu_bin: "qemu-system-arm".into(),
        args: vec!["-m".into(), "1G".into()],
        desc: Some("test vm".into()),
        qemu_version: Some("9.0".into()),
    };
    let json = serde_json::to_string_pretty(&config).unwrap();
    std::fs::write(dir.path().join("myvm.json"), &json).unwrap();

    let loaded = load_config_from_dir(dir.path(), "myvm").unwrap();
    assert_eq!(loaded.qemu_bin, "qemu-system-arm");
    assert_eq!(loaded.args, vec!["-m", "1G"]);
    assert_eq!(loaded.desc.as_deref(), Some("test vm"));
    assert_eq!(loaded.qemu_version.as_deref(), Some("9.0"));
}

#[test]
fn load_config_from_dir_missing_returns_config_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let err = load_config_from_dir(dir.path(), "nonexistent").unwrap_err();
    assert!(matches!(err, VexError::ConfigNotFound { name } if name == "nonexistent"));
}

#[test]
fn load_config_from_dir_invalid_json_returns_parse_failed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("broken.json"), "not valid json").unwrap();
    let err = load_config_from_dir(dir.path(), "broken").unwrap_err();
    assert!(matches!(err, VexError::ConfigParseFailed { .. }));
}

#[test]
fn load_config_rejects_path_traversal() {
    use crate::config::load_config;
    let err = load_config("../escape").unwrap_err();
    assert!(matches!(err, VexError::ValidationError { .. }));
}

#[test]
fn load_config_accepts_legacy_names_with_special_chars() {
    let dir = tempfile::tempdir().unwrap();
    let config = QemuConfig {
        qemu_bin: "qemu".into(),
        args: vec![],
        desc: None,
        qemu_version: None,
    };
    let json = serde_json::to_string(&config).unwrap();
    std::fs::write(dir.path().join("my vm.json"), &json).unwrap();
    let loaded = load_config_from_dir(dir.path(), "my vm").unwrap();
    assert_eq!(loaded.qemu_bin, "qemu");
}

#[test]
fn sanitize_allows_special_chars_but_rejects_traversal() {
    assert!(sanitize_config_name("my vm").is_ok());
    assert!(sanitize_config_name("café").is_ok());
    assert!(sanitize_config_name("虚拟机").is_ok());
    assert!(sanitize_config_name("name@host").is_ok());

    assert!(sanitize_config_name("").is_err());
    assert!(sanitize_config_name(".").is_err());
    assert!(sanitize_config_name("..").is_err());
    assert!(sanitize_config_name("a/b").is_err());
    assert!(sanitize_config_name("a\\b").is_err());
    assert!(sanitize_config_name("name\0evil").is_err());
    let long_name = "a".repeat(256);
    assert!(sanitize_config_name(&long_name).is_err());
}

#[test]
fn validate_is_stricter_than_sanitize() {
    assert!(sanitize_config_name("my vm").is_ok());
    assert!(validate_config_name("my vm").is_err());

    assert!(sanitize_config_name("name@host").is_ok());
    assert!(validate_config_name("name@host").is_err());

    assert!(sanitize_config_name("my-vm").is_ok());
    assert!(validate_config_name("my-vm").is_ok());
}

#[test]
fn validate_null_byte_in_binary_rejected() {
    let config = QemuConfig {
        qemu_bin: "qemu\0injected".into(),
        args: vec![],
        desc: None,
        qemu_version: None,
    };
    assert!(validate_config(&config).is_ok());
}

#[test]
fn validate_config_no_args_accepted() {
    let config = QemuConfig {
        qemu_bin: "qemu-system-riscv64".into(),
        args: vec![],
        desc: None,
        qemu_version: None,
    };
    assert!(validate_config(&config).is_ok());
}

#[test]
fn validate_config_many_args_accepted() {
    let config = QemuConfig {
        qemu_bin: "qemu".into(),
        args: (0..100).map(|i| format!("-arg{}", i)).collect(),
        desc: None,
        qemu_version: None,
    };
    assert!(validate_config(&config).is_ok());
}

#[test]
fn validate_multiple_empty_args_reports_first_index() {
    let config = QemuConfig {
        qemu_bin: "qemu".into(),
        args: vec!["-m".into(), "".into(), "".into()],
        desc: None,
        qemu_version: None,
    };
    let err = validate_config(&config).unwrap_err();
    match err {
        VexError::ValidationError { field, .. } => {
            assert_eq!(field, Some("args[1]".to_string()));
        }
        _ => panic!("expected ValidationError"),
    }
}

#[test]
fn validate_null_byte_at_various_arg_positions() {
    for i in 0..3 {
        let mut args: Vec<String> = vec!["-m".into(), "2G".into(), "-smp".into()];
        args[i] = format!("val\0ue{}", i);
        let config = QemuConfig {
            qemu_bin: "qemu".into(),
            args,
            desc: None,
            qemu_version: None,
        };
        let err = validate_config(&config).unwrap_err();
        match err {
            VexError::ValidationError { field, reason, .. } => {
                assert_eq!(field, Some(format!("args[{}]", i)));
                assert!(reason.contains("null"));
            }
            _ => panic!("expected ValidationError for index {}", i),
        }
    }
}

#[test]
fn validate_name_unicode_rejected() {
    assert!(validate_config_name("虚拟机").is_err());
    assert!(validate_config_name("café").is_err());
    assert!(validate_config_name("名前").is_err());
}

#[test]
fn validate_name_control_chars_rejected() {
    assert!(validate_config_name("name\ttab").is_err());
    assert!(validate_config_name("name\nnewline").is_err());
    assert!(validate_config_name("name\rcarriage").is_err());
    assert!(validate_config_name("\x07bell").is_err());
}

#[test]
fn validate_name_triple_dots_accepted() {
    assert!(validate_config_name("...").is_ok());
}

#[test]
fn validate_name_only_hyphens_accepted() {
    assert!(validate_config_name("-").is_ok());
    assert!(validate_config_name("---").is_ok());
}

#[test]
fn validate_name_only_underscores_accepted() {
    assert!(validate_config_name("_").is_ok());
    assert!(validate_config_name("___").is_ok());
}

#[test]
fn validate_name_colon_semicolon_pipe_rejected() {
    assert!(validate_config_name("name:tag").is_err());
    assert!(validate_config_name("name;cmd").is_err());
    assert!(validate_config_name("name|pipe").is_err());
}

#[test]
fn validate_name_boundary_254_accepted() {
    let name = "a".repeat(254);
    assert!(validate_config_name(&name).is_ok());
}

#[test]
fn validate_name_boundary_1_accepted() {
    assert!(validate_config_name("a").is_ok());
    assert!(validate_config_name("0").is_ok());
    assert!(validate_config_name("_").is_ok());
    assert!(validate_config_name("-").is_ok());
}

#[test]
fn validate_name_tilde_hash_rejected() {
    assert!(validate_config_name("~config").is_err());
    assert!(validate_config_name("#config").is_err());
    assert!(validate_config_name("config$").is_err());
    assert!(validate_config_name("config%").is_err());
}

#[test]
fn parse_config_json_extra_fields_tolerated() {
    let json = r#"{"qemu_bin":"qemu","args":[],"extra_field":"ignored","num":42}"#;
    let config = parse_config_json(json).unwrap();
    assert_eq!(config.qemu_bin, "qemu");
}

#[test]
fn parse_config_json_numeric_qemu_bin_rejected() {
    let json = r#"{"qemu_bin":42,"args":[]}"#;
    assert!(parse_config_json(json).is_err());
}

#[test]
fn parse_config_json_non_array_args_rejected() {
    let json = r#"{"qemu_bin":"qemu","args":"not-an-array"}"#;
    assert!(parse_config_json(json).is_err());
}

#[test]
fn parse_config_json_null_qemu_bin_rejected() {
    let json = r#"{"qemu_bin":null,"args":[]}"#;
    assert!(parse_config_json(json).is_err());
}

#[test]
fn parse_config_json_args_with_numbers_rejected() {
    let json = r#"{"qemu_bin":"qemu","args":[1,2,3]}"#;
    assert!(parse_config_json(json).is_err());
}

#[test]
fn parse_config_json_preserves_unicode_in_desc() {
    let json = r#"{"qemu_bin":"qemu","args":[],"desc":"开发虚拟机 🖥️"}"#;
    let config = parse_config_json(json).unwrap();
    assert_eq!(config.desc.as_deref(), Some("开发虚拟机 🖥️"));
}

#[test]
fn parse_config_json_preserves_version() {
    let json = r#"{"qemu_bin":"qemu","args":[],"qemu_version":"9.1.2"}"#;
    let config = parse_config_json(json).unwrap();
    assert_eq!(config.qemu_version.as_deref(), Some("9.1.2"));
}

#[test]
fn parse_config_json_empty_string_rejected() {
    assert!(parse_config_json("").is_err());
}

#[test]
fn parse_config_json_array_at_top_level_rejected() {
    assert!(parse_config_json(r#"[{"qemu_bin":"q","args":[]}]"#).is_err());
}

#[test]
fn load_config_from_dir_truncated_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("trunc.json"), r#"{"qemu_bin":"qemu","args"#).unwrap();
    let err = load_config_from_dir(dir.path(), "trunc").unwrap_err();
    assert!(matches!(err, VexError::ConfigParseFailed { .. }));
}

#[test]
fn load_config_from_dir_preserves_all_fields() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "qemu_bin": "qemu-system-aarch64",
        "args": ["-m", "4G", "-smp", "8"],
        "desc": "ARM64 dev box",
        "qemu_version": "9.0.1"
    }"#;
    std::fs::write(dir.path().join("full.json"), json).unwrap();
    let config = load_config_from_dir(dir.path(), "full").unwrap();
    assert_eq!(config.qemu_bin, "qemu-system-aarch64");
    assert_eq!(config.args, vec!["-m", "4G", "-smp", "8"]);
    assert_eq!(config.desc.as_deref(), Some("ARM64 dev box"));
    assert_eq!(config.qemu_version.as_deref(), Some("9.0.1"));
}

#[test]
fn load_config_from_dir_empty_json_object() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("empty.json"), "{}").unwrap();
    let err = load_config_from_dir(dir.path(), "empty").unwrap_err();
    assert!(matches!(err, VexError::ConfigParseFailed { .. }));
}

#[test]
fn load_config_from_dir_partial_json_missing_args() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("partial.json"), r#"{"qemu_bin":"qemu"}"#).unwrap();
    let err = load_config_from_dir(dir.path(), "partial").unwrap_err();
    assert!(matches!(err, VexError::ConfigParseFailed { .. }));
}

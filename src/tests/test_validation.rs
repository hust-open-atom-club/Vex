use crate::config::{
    QemuConfig, load_config_from_dir, parse_config_json, validate_config, validate_config_name,
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
fn load_config_validates_name() {
    use crate::config::load_config;
    let err = load_config("../escape").unwrap_err();
    assert!(matches!(err, VexError::ValidationError { .. }));
}

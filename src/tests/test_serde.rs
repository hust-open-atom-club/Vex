use crate::config::QemuConfig;
use crate::remote::PublishedConfig;

#[test]
fn qemu_config_serialize_deserialize_round_trip() {
    let config = QemuConfig {
        qemu_bin: "qemu-system-x86_64".into(),
        args: vec!["-m".into(), "4G".into(), "-smp".into(), "8".into()],
        desc: Some("Production VM".into()),
        qemu_version: Some("9.0.1".into()),
    };
    let json = serde_json::to_string_pretty(&config).unwrap();
    let parsed: QemuConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.qemu_bin, config.qemu_bin);
    assert_eq!(parsed.args, config.args);
    assert_eq!(parsed.desc, config.desc);
    assert_eq!(parsed.qemu_version, config.qemu_version);
}

#[test]
fn qemu_config_minimal_round_trip() {
    let config = QemuConfig {
        qemu_bin: "qemu".into(),
        args: vec![],
        desc: None,
        qemu_version: None,
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: QemuConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.qemu_bin, "qemu");
    assert!(parsed.args.is_empty());
    assert!(parsed.desc.is_none());
    assert!(parsed.qemu_version.is_none());
}

#[test]
fn qemu_config_preserves_empty_string_args() {
    let config = QemuConfig {
        qemu_bin: "qemu".into(),
        args: vec!["".into()],
        desc: None,
        qemu_version: None,
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: QemuConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.args, vec![""]);
}

#[test]
fn qemu_config_preserves_special_chars_in_args() {
    let config = QemuConfig {
        qemu_bin: "qemu".into(),
        args: vec![
            "-drive".into(),
            "file=/path/to/disk with spaces.qcow2,format=qcow2".into(),
            "-netdev".into(),
            "user,id=net0,hostfwd=tcp::2222-:22".into(),
        ],
        desc: Some("VM with 'special' \"chars\"".into()),
        qemu_version: None,
    };
    let json = serde_json::to_string_pretty(&config).unwrap();
    let parsed: QemuConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.args, config.args);
    assert_eq!(parsed.desc, config.desc);
}

#[test]
fn qemu_config_json_field_order_does_not_matter() {
    let json1 = r#"{"args":["-m","2G"],"qemu_bin":"qemu"}"#;
    let json2 = r#"{"qemu_bin":"qemu","args":["-m","2G"]}"#;
    let c1: QemuConfig = serde_json::from_str(json1).unwrap();
    let c2: QemuConfig = serde_json::from_str(json2).unwrap();
    assert_eq!(c1.qemu_bin, c2.qemu_bin);
    assert_eq!(c1.args, c2.args);
}

#[test]
fn published_config_serialize_deserialize_round_trip() {
    let published = PublishedConfig {
        schema_version: 1,
        id: "team".into(),
        name: "demo".into(),
        tag: "v1".into(),
        config: QemuConfig {
            qemu_bin: "qemu-system-arm".into(),
            args: vec!["-m".into(), "1G".into()],
            desc: Some("ARM demo".into()),
            qemu_version: Some("8.2.0".into()),
        },
    };
    let json = serde_json::to_string_pretty(&published).unwrap();
    let parsed: PublishedConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.schema_version, 1);
    assert_eq!(parsed.id, "team");
    assert_eq!(parsed.name, "demo");
    assert_eq!(parsed.tag, "v1");
    assert_eq!(parsed.config.qemu_bin, "qemu-system-arm");
}

#[test]
fn published_config_json_contains_all_fields() {
    let published = PublishedConfig {
        schema_version: 1,
        id: "org".into(),
        name: "cfg".into(),
        tag: "latest".into(),
        config: QemuConfig {
            qemu_bin: "qemu".into(),
            args: vec![],
            desc: None,
            qemu_version: None,
        },
    };
    let json = serde_json::to_string(&published).unwrap();
    assert!(json.contains("\"schema_version\":1"));
    assert!(json.contains("\"id\":\"org\""));
    assert!(json.contains("\"name\":\"cfg\""));
    assert!(json.contains("\"tag\":\"latest\""));
    assert!(json.contains("\"qemu_bin\":\"qemu\""));
}

#[test]
fn qemu_config_clone_produces_equal_value() {
    let config = QemuConfig {
        qemu_bin: "qemu-system-riscv64".into(),
        args: vec!["-m".into(), "512M".into()],
        desc: Some("RISC-V".into()),
        qemu_version: Some("9.1.0".into()),
    };
    let cloned = config.clone();
    assert_eq!(cloned.qemu_bin, config.qemu_bin);
    assert_eq!(cloned.args, config.args);
    assert_eq!(cloned.desc, config.desc);
    assert_eq!(cloned.qemu_version, config.qemu_version);
}

#[test]
fn qemu_config_debug_format() {
    let config = QemuConfig {
        qemu_bin: "qemu".into(),
        args: vec!["-m".into(), "2G".into()],
        desc: None,
        qemu_version: None,
    };
    let debug = format!("{:?}", config);
    assert!(debug.contains("qemu"));
    assert!(debug.contains("2G"));
}

#[test]
fn published_config_with_legacy_format_compatible() {
    let legacy_json = r#"{
        "qemu_bin": "qemu-system-x86_64",
        "args": ["-m", "2G"],
        "desc": "legacy config"
    }"#;
    let config: QemuConfig = serde_json::from_str(legacy_json).unwrap();
    assert_eq!(config.qemu_bin, "qemu-system-x86_64");
    assert!(config.qemu_version.is_none());
}

#[test]
fn qemu_config_with_unicode_desc_round_trip() {
    let config = QemuConfig {
        qemu_bin: "qemu".into(),
        args: vec![],
        desc: Some("开发环境 — développement — 開発".into()),
        qemu_version: None,
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: QemuConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.desc, config.desc);
}

#[test]
fn qemu_config_large_args_round_trip() {
    let args: Vec<String> = (0..200).map(|i| format!("-arg{}=value{}", i, i)).collect();
    let config = QemuConfig {
        qemu_bin: "qemu".into(),
        args: args.clone(),
        desc: None,
        qemu_version: None,
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: QemuConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.args.len(), 200);
    assert_eq!(parsed.args, args);
}

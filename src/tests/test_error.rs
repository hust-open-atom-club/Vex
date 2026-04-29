use std::error::Error;
use std::path::PathBuf;

use crate::error::VexError;

#[test]
fn config_not_found_display() {
    let err = VexError::ConfigNotFound {
        name: "my-vm".into(),
    };
    assert_eq!(err.to_string(), "Configuration 'my-vm' not found");
}

#[test]
fn config_not_found_empty_name() {
    let err = VexError::ConfigNotFound {
        name: String::new(),
    };
    assert_eq!(err.to_string(), "Configuration '' not found");
}

#[test]
fn config_already_exists_display() {
    let err = VexError::ConfigAlreadyExists {
        name: "test".into(),
    };
    assert_eq!(err.to_string(), "Configuration 'test' already exists");
}

#[test]
fn config_parse_failed_has_source() {
    let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
    let err = VexError::ConfigParseFailed { source: json_err };
    assert!(err.source().is_some());
    assert!(err.to_string().contains("parse"));
}

#[test]
fn config_serialize_failed_has_source() {
    let json_err = serde_json::from_str::<serde_json::Value>("bad").unwrap_err();
    let err = VexError::ConfigSerializeFailed { source: json_err };
    assert!(err.source().is_some());
    assert!(err.to_string().contains("serialize"));
}

#[test]
fn validation_error_with_field() {
    let err = VexError::ValidationError {
        field: Some("qemu_bin".into()),
        reason: "cannot be empty".into(),
    };
    let msg = err.to_string();
    assert!(msg.contains("qemu_bin"));
    assert!(msg.contains("cannot be empty"));
}

#[test]
fn validation_error_without_field() {
    let err = VexError::ValidationError {
        field: None,
        reason: "general failure".into(),
    };
    let msg = err.to_string();
    assert!(msg.contains("general failure"));
    assert!(!msg.contains("None"));
}

#[test]
fn io_error_display_includes_path_and_operation() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = VexError::IoError {
        path: PathBuf::from("/tmp/test.json"),
        operation: "read".into(),
        source: io_err,
    };
    let msg = err.to_string();
    assert!(msg.contains("/tmp/test.json"));
    assert!(msg.contains("read"));
    assert!(err.source().is_some());
}

#[test]
fn qemu_launch_failed_has_source() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let err = VexError::QemuLaunchFailed {
        binary: "qemu-system-x86_64".into(),
        source: io_err,
    };
    assert!(err.to_string().contains("qemu-system-x86_64"));
    assert!(err.source().is_some());
}

#[test]
fn qemu_exit_error_with_code() {
    let err = VexError::QemuExitError {
        binary: "qemu-system-arm".into(),
        exit_code: Some(1),
    };
    let msg = err.to_string();
    assert!(msg.contains("1"));
    assert!(msg.contains("qemu-system-arm"));
}

#[test]
fn qemu_exit_error_without_code() {
    let err = VexError::QemuExitError {
        binary: "qemu".into(),
        exit_code: None,
    };
    let msg = err.to_string();
    assert!(msg.contains("unknown"));
}

#[test]
fn remote_not_configured_display() {
    let err = VexError::RemoteNotConfigured {
        env_var: "VEX_REMOTE_URL".into(),
    };
    assert!(err.to_string().contains("VEX_REMOTE_URL"));
}

#[test]
fn remote_spec_invalid_display() {
    let err = VexError::RemoteSpecInvalid {
        input: "bad-spec".into(),
        reason: "missing slash".into(),
    };
    let msg = err.to_string();
    assert!(msg.contains("bad-spec"));
    assert!(msg.contains("missing slash"));
}

#[test]
fn git_command_failed_preserves_stderr() {
    let err = VexError::GitCommandFailed {
        args: "push origin main".into(),
        stderr: "permission denied".into(),
        stdout: String::new(),
        exit_code: Some(128),
    };
    let msg = err.to_string();
    assert!(msg.contains("push origin main"));
    assert!(msg.contains("permission denied"));
}

#[test]
fn remote_config_not_found_display() {
    let err = VexError::RemoteConfigNotFound {
        id: "team".into(),
        name: "demo".into(),
        tag: "v1".into(),
    };
    assert!(err.to_string().contains("team/demo:v1"));
}

#[test]
fn editor_failed_with_exit_code() {
    let err = VexError::EditorFailed {
        editor: "vim".into(),
        exit_code: Some(1),
    };
    let msg = err.to_string();
    assert!(msg.contains("vim"));
    assert!(msg.contains("1"));
}

#[test]
fn editor_failed_without_exit_code() {
    let err = VexError::EditorFailed {
        editor: "nano".into(),
        exit_code: None,
    };
    let msg = err.to_string();
    assert!(msg.contains("nano"));
    assert!(msg.contains("unknown"));
}

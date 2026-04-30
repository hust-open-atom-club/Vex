use std::error::Error;
use std::io;
use std::path::PathBuf;

use crate::error::VexError;

#[test]
fn from_io_error_produces_io_variant() {
    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
    let vex_err: VexError = io_err.into();
    assert!(matches!(vex_err, VexError::IoError { .. }));
    assert!(vex_err.to_string().contains("<unknown>"));
}

#[test]
fn from_serde_error_produces_parse_variant() {
    let json_err = serde_json::from_str::<serde_json::Value>("!!!").unwrap_err();
    let vex_err: VexError = json_err.into();
    assert!(matches!(vex_err, VexError::ConfigParseFailed { .. }));
    assert!(vex_err.source().is_some());
}

#[test]
fn git_command_failed_display_includes_stdout() {
    let err = VexError::GitCommandFailed {
        args: "status".into(),
        stderr: String::new(),
        stdout: "on branch main".into(),
        exit_code: Some(1),
    };
    let msg = err.to_string();
    assert!(msg.contains("on branch main"));
}

#[test]
fn git_command_failed_display_with_both_stderr_and_stdout() {
    let err = VexError::GitCommandFailed {
        args: "push".into(),
        stderr: "rejected".into(),
        stdout: "Everything up-to-date".into(),
        exit_code: Some(1),
    };
    let msg = err.to_string();
    assert!(msg.contains("rejected"));
    assert!(msg.contains("Everything up-to-date"));
}

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

#[test]
fn git_command_failed_no_exit_code() {
    let err = VexError::GitCommandFailed {
        args: "fetch origin".into(),
        stderr: "network error".into(),
        stdout: String::new(),
        exit_code: None,
    };
    let msg = err.to_string();
    assert!(msg.contains("unknown"));
    assert!(msg.contains("fetch origin"));
    assert!(msg.contains("network error"));
}

#[test]
fn git_command_failed_empty_everything() {
    let err = VexError::GitCommandFailed {
        args: String::new(),
        stderr: String::new(),
        stdout: String::new(),
        exit_code: None,
    };
    let msg = err.to_string();
    assert!(msg.contains("unknown"));
    assert!(!msg.contains(":"));
}

#[test]
fn git_command_failed_only_stderr_no_stdout() {
    let err = VexError::GitCommandFailed {
        args: "clone".into(),
        stderr: "fatal: repo not found".into(),
        stdout: String::new(),
        exit_code: Some(128),
    };
    let msg = err.to_string();
    assert!(msg.contains("128"));
    assert!(msg.contains("fatal: repo not found"));
    assert!(!msg.contains("()")); // empty stdout should not produce empty parens
}

#[test]
fn qemu_exit_error_signal_codes() {
    for code in [2, 127, 137, 139, 255] {
        let err = VexError::QemuExitError {
            binary: "qemu".into(),
            exit_code: Some(code),
        };
        assert!(err.to_string().contains(&code.to_string()));
    }
}

#[test]
fn io_error_various_kinds() {
    let kinds = [
        io::ErrorKind::NotFound,
        io::ErrorKind::PermissionDenied,
        io::ErrorKind::AlreadyExists,
        io::ErrorKind::InvalidInput,
    ];
    for kind in kinds {
        let io_err = io::Error::new(kind, "test");
        let err = VexError::IoError {
            path: PathBuf::from("/some/path"),
            operation: "test op".into(),
            source: io_err,
        };
        assert!(err.source().is_some());
        assert!(err.to_string().contains("/some/path"));
    }
}

#[test]
fn remote_config_not_found_empty_segments() {
    let err = VexError::RemoteConfigNotFound {
        id: String::new(),
        name: String::new(),
        tag: String::new(),
    };
    assert!(err.to_string().contains("/:"));
}

#[test]
fn from_io_error_sets_unknown_path_and_generic_operation() {
    let io_err = io::Error::new(io::ErrorKind::Other, "something");
    let vex_err: VexError = io_err.into();
    match vex_err {
        VexError::IoError {
            path, operation, ..
        } => {
            assert_eq!(path, PathBuf::from("<unknown>"));
            assert_eq!(operation, "io operation");
        }
        _ => panic!("expected IoError"),
    }
}

#[test]
fn validation_error_empty_reason() {
    let err = VexError::ValidationError {
        field: Some("test".into()),
        reason: String::new(),
    };
    let msg = err.to_string();
    assert!(msg.contains("test"));
}

#[test]
fn all_error_variants_implement_debug() {
    let io_err = io::Error::new(io::ErrorKind::Other, "test");
    let json_err = serde_json::from_str::<serde_json::Value>("!").unwrap_err();
    let json_err2 = serde_json::from_str::<serde_json::Value>("!").unwrap_err();
    let errors: Vec<VexError> = vec![
        VexError::ConfigNotFound { name: "x".into() },
        VexError::ConfigAlreadyExists { name: "x".into() },
        VexError::ConfigParseFailed { source: json_err },
        VexError::ConfigSerializeFailed { source: json_err2 },
        VexError::ValidationError {
            field: None,
            reason: "r".into(),
        },
        VexError::IoError {
            path: PathBuf::from("p"),
            operation: "o".into(),
            source: io_err,
        },
        VexError::RemoteNotConfigured {
            env_var: "V".into(),
        },
        VexError::RemoteSpecInvalid {
            input: "i".into(),
            reason: "r".into(),
        },
        VexError::GitCommandFailed {
            args: "a".into(),
            stderr: "s".into(),
            stdout: "o".into(),
            exit_code: Some(1),
        },
        VexError::RemoteConfigNotFound {
            id: "i".into(),
            name: "n".into(),
            tag: "t".into(),
        },
        VexError::EditorFailed {
            editor: "e".into(),
            exit_code: None,
        },
    ];
    for err in &errors {
        let debug = format!("{:?}", err);
        assert!(!debug.is_empty());
    }
}

#[test]
fn config_not_found_with_special_name() {
    let err = VexError::ConfigNotFound {
        name: "my-vm_2.0".into(),
    };
    assert_eq!(err.to_string(), "Configuration 'my-vm_2.0' not found");
}

#[test]
fn config_already_exists_with_special_name() {
    let err = VexError::ConfigAlreadyExists {
        name: "vm.backup-2024".into(),
    };
    assert!(err.to_string().contains("vm.backup-2024"));
}

use crate::commands::exec::{check_resource_files, substitute_params, substitute_resources};
use crate::config::{ResourceKind, ResourceRef};
use crate::error::VexError;
use std::collections::HashMap;

#[test]
fn test_substitute_single_var() {
    let env: HashMap<String, String> = [("TEST_VAR".into(), "value".into())].into();
    let args = vec!["${TEST_VAR}".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["value"]);
}

#[test]
fn test_substitute_multiple_vars() {
    let env: HashMap<String, String> = [
        ("VAR1".into(), "hello".into()),
        ("VAR2".into(), "world".into()),
    ]
    .into();
    let args = vec!["${VAR1} ${VAR2}".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["hello world"]);
}

#[test]
fn test_substitute_undefined_var() {
    let env: HashMap<String, String> = HashMap::new();
    let args = vec!["${UNDEFINED_VAR}".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["${UNDEFINED_VAR}"]);
}

#[test]
fn test_substitute_mixed() {
    let env: HashMap<String, String> = [("DEFINED".into(), "yes".into())].into();
    let args = vec!["prefix_${DEFINED}_${UNDEFINED}_suffix".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["prefix_yes_${UNDEFINED}_suffix"]);
}

#[test]
fn test_substitute_no_vars() {
    let env: HashMap<String, String> = HashMap::new();
    let args = vec!["no_vars_here".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["no_vars_here"]);
}

#[test]
fn test_substitute_empty_args() {
    let env: HashMap<String, String> = HashMap::new();
    let args: Vec<String> = vec![];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, Vec::<String>::new());
}

#[test]
fn test_substitute_multiple_args() {
    let env: HashMap<String, String> = [("PATH_VAR".into(), "/usr/bin".into())].into();
    let args = vec![
        "${PATH_VAR}/qemu".to_string(),
        "-m".to_string(),
        "2048".to_string(),
    ];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["/usr/bin/qemu", "-m", "2048"]);
}

#[test]
fn test_substitute_nested_braces() {
    let env: HashMap<String, String> = [("OUTER".into(), "value".into())].into();
    let args = vec!["${OUTER}".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["value"]);
}

#[test]
fn test_substitute_malformed_unclosed_brace_passes_through() {
    let env: HashMap<String, String> = HashMap::new();
    let args = vec!["disk=${".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["disk=${"]);
}

#[test]
fn test_substitute_dollar_without_braces_passes_through() {
    let env: HashMap<String, String> = [("VAR".into(), "val".into())].into();
    let args = vec!["$VAR".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["$VAR"]);
}

#[test]
fn test_substitute_bare_dollar_sign() {
    let env: HashMap<String, String> = HashMap::new();
    let args = vec!["cost=$100".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["cost=$100"]);
}

#[test]
fn test_substitute_empty_var_name() {
    let env: HashMap<String, String> = HashMap::new();
    let args = vec!["${}".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["${}"]);
}

#[test]
fn test_substitute_var_name_with_spaces() {
    let env: HashMap<String, String> = [("MY VAR".into(), "value".into())].into();
    let args = vec!["${MY VAR}".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["value"]);
}

#[test]
fn test_substitute_adjacent_vars() {
    let env: HashMap<String, String> =
        [("A".into(), "hello".into()), ("B".into(), "world".into())].into();
    let args = vec!["${A}${B}".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["helloworld"]);
}

#[test]
fn test_substitute_same_var_multiple_times() {
    let env: HashMap<String, String> = [("X".into(), "abc".into())].into();
    let args = vec!["${X}-${X}-${X}".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["abc-abc-abc"]);
}

#[test]
fn test_substitute_value_containing_dollar_brace() {
    let env: HashMap<String, String> = [("VAR".into(), "${OTHER}".into())].into();
    let args = vec!["${VAR}".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["${OTHER}"]);
}

#[test]
fn test_substitute_very_long_var_name() {
    let long_name = "A".repeat(1000);
    let env: HashMap<String, String> = [(long_name.clone(), "found".into())].into();
    let args = vec![format!("${{{}}}", long_name)];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["found"]);
}

#[test]
fn test_substitute_preserves_arg_order() {
    let env: HashMap<String, String> = HashMap::new();
    let args = vec![
        "first".to_string(),
        "second".to_string(),
        "third".to_string(),
    ];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["first", "second", "third"]);
}

#[test]
fn test_substitute_at_start_middle_end() {
    let env: HashMap<String, String> = [
        ("START".into(), "s".into()),
        ("MID".into(), "m".into()),
        ("END".into(), "e".into()),
    ]
    .into();
    let args = vec![
        "${START}xxx".to_string(),
        "xxx${MID}xxx".to_string(),
        "xxx${END}".to_string(),
    ];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["sxxx", "xxxmxxx", "xxxe"]);
}

#[test]
fn test_substitute_params_with_map_basic() {
    use crate::commands::exec::substitute_params_with_map;
    let env: HashMap<String, String> = [("HOME".into(), "/home/user".into())].into();
    let args = vec!["${HOME}/images/disk.qcow2".to_string()];
    let result = substitute_params_with_map(&args, &env);
    assert_eq!(result, vec!["/home/user/images/disk.qcow2"]);
}

#[test]
fn test_substitute_closing_brace_in_text() {
    let env: HashMap<String, String> = HashMap::new();
    let args = vec!["text}more".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["text}more"]);
}

#[test]
fn test_substitute_double_dollar_sign() {
    let env: HashMap<String, String> = HashMap::new();
    let args = vec!["$${NOT_A_VAR}".to_string()];
    let result = substitute_params(&args, |k| env.get(k).cloned());
    assert_eq!(result, vec!["$${NOT_A_VAR}"]);
}

fn make_resource(path: &str, kind: ResourceKind) -> ResourceRef {
    ResourceRef {
        path: path.to_string(),
        kind,
        sha256: None,
        size: None,
        url: None,
    }
}

#[test]
fn test_substitute_resources_replaces_known_key() {
    let mut resources = HashMap::new();
    resources.insert(
        "disk".to_string(),
        make_resource("/tmp/x.img", ResourceKind::Image),
    );
    let args = vec![
        "-drive".to_string(),
        "file=${res:disk},format=raw".to_string(),
    ];
    let result = substitute_resources(&args, &resources).unwrap();
    assert_eq!(result[0], "-drive");
    assert_eq!(result[1], "file=/tmp/x.img,format=raw");
}

#[test]
fn test_substitute_resources_handles_multiple_occurrences_in_one_arg() {
    let mut resources = HashMap::new();
    resources.insert("a".to_string(), make_resource("/A", ResourceKind::Image));
    resources.insert("b".to_string(), make_resource("/B", ResourceKind::Firmware));
    let args = vec!["${res:a}+${res:b}+${res:a}".to_string()];
    let result = substitute_resources(&args, &resources).unwrap();
    assert_eq!(result, vec!["/A+/B+/A"]);
}

#[test]
fn test_substitute_resources_unknown_key_errors() {
    let resources: HashMap<String, ResourceRef> = HashMap::new();
    let args = vec!["${res:ghost}".to_string()];
    let err = substitute_resources(&args, &resources).unwrap_err();
    match err {
        VexError::UnknownResourceReference { key, arg_index } => {
            assert_eq!(key, "ghost");
            assert_eq!(arg_index, 0);
        }
        other => panic!("expected UnknownResourceReference, got {:?}", other),
    }
}

#[test]
fn test_substitute_resources_arg_index_reflects_position() {
    let resources: HashMap<String, ResourceRef> = HashMap::new();
    let args = vec![
        "ok".to_string(),
        "still-ok".to_string(),
        "${res:bad}".to_string(),
    ];
    let err = substitute_resources(&args, &resources).unwrap_err();
    match err {
        VexError::UnknownResourceReference { arg_index, .. } => assert_eq!(arg_index, 2),
        other => panic!("expected UnknownResourceReference, got {:?}", other),
    }
}

#[test]
fn test_substitute_resources_invalid_key_pattern_is_left_alone() {
    let resources: HashMap<String, ResourceRef> = HashMap::new();
    let args = vec![
        "${res:my-disk}".to_string(),
        "${res:1x}".to_string(),
        "${res:}".to_string(),
    ];
    let result = substitute_resources(&args, &resources).unwrap();
    assert_eq!(result[0], "${res:my-disk}");
    assert_eq!(result[1], "${res:1x}");
    assert_eq!(result[2], "${res:}");
}

#[test]
fn test_substitute_resources_does_not_touch_env_placeholders() {
    let mut resources = HashMap::new();
    resources.insert("disk".to_string(), make_resource("/D", ResourceKind::Image));
    let args = vec!["${HOME}".to_string(), "${res:disk}".to_string()];
    let result = substitute_resources(&args, &resources).unwrap();
    assert_eq!(result[0], "${HOME}");
    assert_eq!(result[1], "/D");
}

#[test]
fn test_check_resource_files_ok_for_existing_file() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let mut resources = HashMap::new();
    resources.insert(
        "disk".to_string(),
        make_resource(tmp.path().to_str().unwrap(), ResourceKind::Image),
    );
    assert!(check_resource_files(&resources).is_ok());
}

#[test]
fn test_check_resource_files_errors_for_missing() {
    let mut resources = HashMap::new();
    let missing = "/tmp/definitely-not-exists-vex-test-zzz-09182374";
    resources.insert(
        "disk".to_string(),
        make_resource(missing, ResourceKind::Image),
    );
    let err = check_resource_files(&resources).unwrap_err();
    match err {
        VexError::ResourceFileNotFound { key, path } => {
            assert_eq!(key, "disk");
            assert_eq!(path, std::path::PathBuf::from(missing));
        }
        other => panic!("expected ResourceFileNotFound, got {:?}", other),
    }
}

mod prepare_command_tests {
    use super::make_resource;
    use crate::commands::exec::prepare_command;
    use crate::config::{QemuConfig, ResourceKind};
    use crate::error::VexError;
    use std::collections::HashMap;

    fn base_config(args: Vec<String>) -> QemuConfig {
        QemuConfig {
            qemu_bin: "/usr/bin/qemu-system-x86_64".to_string(),
            args,
            desc: None,
            qemu_version: None,
            resources: HashMap::new(),
        }
    }

    #[test]
    fn prepare_command_sets_correct_binary() {
        let config = base_config(vec!["-m".to_string(), "1024".to_string()]);
        let prepared = prepare_command(&config, false).unwrap();
        assert_eq!(
            prepared.command.get_program().to_str().unwrap(),
            "/usr/bin/qemu-system-x86_64"
        );
    }

    #[test]
    fn prepare_command_substitutes_env_vars() {
        let var_name = "VEX_TEST_PREPARE_ENV_VAR_XYZ";
        // SAFETY: test sets and unsets a uniquely-named env var; no other
        // test in this binary references this variable.
        unsafe {
            std::env::set_var(var_name, "/opt/disks/x.img");
        }
        let config = base_config(vec![format!("file=${{{}}}", var_name)]);
        let prepared = prepare_command(&config, false).unwrap();
        // SAFETY: cleanup of the same uniquely-named var set above.
        unsafe {
            std::env::remove_var(var_name);
        }
        assert_eq!(prepared.final_args, vec!["file=/opt/disks/x.img"]);
    }

    #[test]
    fn prepare_command_substitutes_resources() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let tmp_path = tmp.path().to_str().unwrap().to_string();
        let mut config = base_config(vec!["file=${res:k}".to_string()]);
        config.resources.insert(
            "k".to_string(),
            make_resource(&tmp_path, ResourceKind::Image),
        );
        let prepared = prepare_command(&config, false).unwrap();
        assert_eq!(prepared.final_args, vec![format!("file={}", tmp_path)]);
    }

    #[test]
    fn prepare_command_unknown_resource_reference() {
        let config = base_config(vec!["${res:nonexistent}".to_string()]);
        let err = prepare_command(&config, false).unwrap_err();
        match err {
            VexError::UnknownResourceReference { key, .. } => {
                assert_eq!(key, "nonexistent");
            }
            other => panic!("expected UnknownResourceReference, got {:?}", other),
        }
    }

    #[test]
    fn prepare_command_missing_resource_file_errors() {
        let missing = "/nonexistent/path/vex-prepare-test-abc-999";
        let mut config = base_config(vec!["file=${res:k}".to_string()]);
        config
            .resources
            .insert("k".to_string(), make_resource(missing, ResourceKind::Image));
        let err = prepare_command(&config, false).unwrap_err();
        match err {
            VexError::ResourceFileNotFound { key, path } => {
                assert_eq!(key, "k");
                assert_eq!(path, std::path::PathBuf::from(missing));
            }
            other => panic!("expected ResourceFileNotFound, got {:?}", other),
        }
    }

    #[test]
    fn prepare_command_appends_debug_flags() {
        let config = base_config(vec!["-m".to_string(), "512".to_string()]);
        let prepared = prepare_command(&config, true).unwrap();
        let n = prepared.final_args.len();
        assert!(n >= 2);
        assert_eq!(&prepared.final_args[n - 2..], &["-s", "-S"]);
    }

    #[test]
    fn prepare_command_no_debug_flags_when_false() {
        let config = base_config(vec!["-m".to_string(), "512".to_string()]);
        let prepared = prepare_command(&config, false).unwrap();
        assert!(!prepared.final_args.iter().any(|a| a == "-s" || a == "-S"));
    }
}

use crate::commands::exec::substitute_params;
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

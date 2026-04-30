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

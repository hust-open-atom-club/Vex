use crate::commands::exec::substitute_params;

#[test]
fn test_substitute_single_var() {
    unsafe { std::env::set_var("TEST_VAR", "value") };
    let args = vec!["${TEST_VAR}".to_string()];
    let result = substitute_params(&args);
    assert_eq!(result, vec!["value"]);
}

#[test]
fn test_substitute_multiple_vars() {
    unsafe {
        std::env::set_var("VAR1", "hello");
        std::env::set_var("VAR2", "world");
    }
    let args = vec!["${VAR1} ${VAR2}".to_string()];
    let result = substitute_params(&args);
    assert_eq!(result, vec!["hello world"]);
}

#[test]
fn test_substitute_undefined_var() {
    unsafe { std::env::remove_var("UNDEFINED_VAR") };
    let args = vec!["${UNDEFINED_VAR}".to_string()];
    let result = substitute_params(&args);
    assert_eq!(result, vec!["${UNDEFINED_VAR}"]);
}

#[test]
fn test_substitute_mixed() {
    unsafe {
        std::env::set_var("DEFINED", "yes");
        std::env::remove_var("UNDEFINED");
    }
    let args = vec!["prefix_${DEFINED}_${UNDEFINED}_suffix".to_string()];
    let result = substitute_params(&args);
    assert_eq!(result, vec!["prefix_yes_${UNDEFINED}_suffix"]);
}

#[test]
fn test_substitute_no_vars() {
    let args = vec!["no_vars_here".to_string()];
    let result = substitute_params(&args);
    assert_eq!(result, vec!["no_vars_here"]);
}

#[test]
fn test_substitute_empty_args() {
    let args: Vec<String> = vec![];
    let result = substitute_params(&args);
    assert_eq!(result, Vec::<String>::new());
}

#[test]
fn test_substitute_multiple_args() {
    unsafe { std::env::set_var("PATH_VAR", "/usr/bin") };
    let args = vec![
        "${PATH_VAR}/qemu".to_string(),
        "-m".to_string(),
        "2048".to_string(),
    ];
    let result = substitute_params(&args);
    assert_eq!(result, vec!["/usr/bin/qemu", "-m", "2048"]);
}

#[test]
fn test_substitute_nested_braces() {
    unsafe { std::env::set_var("OUTER", "value") };
    let args = vec!["${OUTER}".to_string()];
    let result = substitute_params(&args);
    assert_eq!(result, vec!["value"]);
}

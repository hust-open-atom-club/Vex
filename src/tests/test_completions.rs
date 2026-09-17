use clap::CommandFactory;
use escargot::CargoBuild;

use crate::commands::Cli;

fn vex_bin() -> escargot::CargoRun {
    CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap()
}

#[test]
fn completions_bash() {
    let output = vex_bin()
        .command()
        .args(["completions", "bash"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("_vex"));
}

#[test]
fn completions_zsh() {
    let output = vex_bin()
        .command()
        .args(["completions", "zsh"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vex") || stdout.contains("_vex"));
}

#[test]
fn zsh_dynamic_completion_lists_all_cli_subcommands() {
    let output = vex_bin()
        .command()
        .args(["completions", "zsh"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let command_block = stdout
        .split("cmds=(")
        .nth(1)
        .and_then(|rest| rest.split_once("\n            )\n"))
        .map(|(block, _)| block)
        .expect("Zsh dynamic completion command block");
    let actual: Vec<_> = command_block
        .lines()
        .filter_map(|line| line.trim().strip_prefix('"'))
        .filter_map(|line| line.split(':').next())
        .map(str::to_owned)
        .collect();
    let expected: Vec<_> = Cli::command()
        .get_subcommands()
        .map(|command| command.get_name().to_owned())
        .collect();

    assert_eq!(actual, expected);
}

#[test]
fn completions_fish() {
    let output = vex_bin()
        .command()
        .args(["completions", "fish"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vex") || stdout.contains("complete"));
}

#[test]
fn completions_powershell() {
    let output = vex_bin()
        .command()
        .args(["completions", "powershell"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty());
}

#[test]
fn completions_elvish() {
    let output = vex_bin()
        .command()
        .args(["completions", "elvish"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty());
}

#[test]
fn completions_invalid_shell_rejected() {
    let output = vex_bin()
        .command()
        .args(["completions", "notashell"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

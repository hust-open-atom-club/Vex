use escargot::CargoBuild;

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

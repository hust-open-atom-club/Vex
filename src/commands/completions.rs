use anyhow::Result;
use clap::{Args, CommandFactory};
use clap_complete::{Shell, generate};
use std::io;

use crate::commands::Cli;

#[derive(Args)]
#[clap(about = "Generate shell completion scripts")]
pub struct CompletionsArgs {
    #[arg(help = "Shell type (bash, zsh, fish, powershell, elvish)")]
    pub shell: Shell,
}

pub fn completions_command(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    // bin_name = Vex
    let bin_name = cmd.get_name().to_string();
    println!("Generating completion script for {}", bin_name);
    // Generate base completion script
    generate(shell, &mut cmd, bin_name.clone(), &mut io::stdout());

    // Add dynamic completion for bash, zsh, fish
    match shell {
        Shell::Bash => print_bash_dynamic_completion(&bin_name),
        Shell::Zsh => print_zsh_dynamic_completion(&bin_name),
        Shell::Fish => print_fish_dynamic_completion(&bin_name),
        _ => {
            eprintln!(
                "# Note: Dynamic config name completion not yet implemented for {:?}",
                shell
            );
        }
    }

    Ok(())
}

fn print_bash_dynamic_completion(bin_name: &str) {
    println!(
        r#"
# === Dynamic configuration name completion ===
# Extract configuration names from vex list output
_vex_get_configs() {{
    {bin_name} list 2>/dev/null | grep ' - ' | awk '{{print $1}}'
}}

# Save original completion function
_vex_original=$(declare -f _vex)
eval "${{_vex_original//_vex()/_vex_base()}}"

# Enhanced completion function
_vex() {{
    local cur prev subcmd
    COMPREPLY=()
    cur="${{COMP_WORDS[COMP_CWORD]}}"
    prev="${{COMP_WORDS[COMP_CWORD-1]}}"

    # Check if configuration name completion is needed
    if [[ ${{COMP_CWORD}} -ge 2 ]]; then
        subcmd="${{COMP_WORDS[1]}}"
        case "$subcmd" in
            exec|rm)
                # First argument for exec and rm is configuration name
                if [[ ${{COMP_CWORD}} -eq 2 ]]; then
                    COMPREPLY=( $(compgen -W "$(_vex_get_configs)" -- "${{cur}}") )
                    return 0
                fi
                ;;
            rename)
                # First argument for rename is old configuration name
                if [[ ${{COMP_CWORD}} -eq 2 ]]; then
                    COMPREPLY=( $(compgen -W "$(_vex_get_configs)" -- "${{cur}}") )
                    return 0
                fi
                ;;
        esac
    fi

    # Fall back to base completion
    _vex_base
}}
"#,
        bin_name = bin_name
    );
}

fn print_zsh_dynamic_completion(bin_name: &str) {
    println!(
        r#"
# === Zsh dynamic configuration name completion ===
_vex_configs() {{
    local configs
    configs=($({bin_name} list 2>/dev/null | grep ' - ' | awk '{{print $1}}'))
    _describe 'configurations' configs
}}

# Enhanced _vex function
_vex() {{
    local line state

    _arguments -C \
        "1: :->cmds" \
        "*::arg:->args"

    case "$state" in
        cmds)
            _values "vex command" \
                "save[Save QEMU configuration]" \
                "rename[Rename a saved QEMU configuration]" \
                "rm[Remove a saved QEMU configuration]" \
                "list[List all saved QEMU configurations]" \
                "exec[Execute a saved QEMU configuration]" \
                "completions[Generate shell completion scripts]"
            ;;
        args)
            case $line[1] in
                exec|rm)
                    _vex_configs
                    ;;
                rename)
                    if [[ $CURRENT -eq 2 ]]; then
                        _vex_configs
                    fi
                    ;;
            esac
            ;;
    esac
}}
"#,
        bin_name = bin_name
    );
}

fn print_fish_dynamic_completion(bin_name: &str) {
    println!(
        r#"
# === Fish dynamic configuration name completion ===
function __vex_configs
    {bin_name} list 2>/dev/null | grep ' - ' | awk '{{print $1}}'
end

# Add configuration name completion for exec command
complete -c vex -n "__fish_seen_subcommand_from exec" -a "(__vex_configs)" -d "Configuration name"

# Add configuration name completion for rm command
complete -c vex -n "__fish_seen_subcommand_from rm" -a "(__vex_configs)" -d "Configuration name"

# Add configuration name completion for rename command (first argument)
complete -c vex -n "__fish_seen_subcommand_from rename; and not __fish_seen_subcommand_from (__vex_configs)" -a "(__vex_configs)" -d "Old configuration name"
"#,
        bin_name = bin_name
    );
}

use clap::Args;

use crate::config::{config_file, load_config};
use crate::error::VexResult;

#[derive(Args, Debug)]
pub struct PrintArgs {
    pub name: String,
}

pub fn print_command(name: String) -> VexResult<()> {
    let config = load_config(&name)?;
    let config_path = config_file(&name)?;

    println!("Configuration: {}", name);
    println!("{}", "=".repeat(60));
    println!();

    if let Some(desc) = &config.desc {
        println!("Description:");
        println!("  {}", desc);
        println!();
    }

    println!("QEMU Binary:");
    println!("  {}", config.qemu_bin);
    println!();

    println!("Startup Arguments:");
    if config.args.is_empty() {
        println!("  (no arguments)");
    } else {
        for (i, arg) in config.args.iter().enumerate() {
            println!("  [{}] {}", i, arg);
        }
    }
    println!();

    println!("Full Command:");
    let full_command = format!("{} {}", config.qemu_bin, config.args.join(" "));
    println!("  {}", full_command);
    println!();

    println!("Configuration File:");
    println!("  {:?}", config_path);

    Ok(())
}

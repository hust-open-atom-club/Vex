use clap::Parser;

/// Vex - QEMU auxiliary command-line tool, simplifying startup parameter management
#[derive(Parser, Debug)]
#[command(
    author, 
    version, 
    about, 
    long_about = "Vex is a command-line tool that helps you manage QEMU virtual machine configurations.\n\
    You can save complex QEMU startup parameters as named configurations and easily execute them later.\n\
    This eliminates the need to remember or type long QEMU command lines repeatedly.\n\n"
)]
pub enum Vex {
    /// Save QEMU startup parameters as a configuration
    /// 
    /// Format: vex save [OPTIONS] <NAME> <QEMU_BIN> [QEMU_ARGS]...
    /// 
    /// This command saves a QEMU configuration that can be executed later.
    /// The configuration includes the QEMU binary path and all startup arguments.
    /// 
    /// Examples:
    /// 
    ///   vex save my-linux qemu-system-x86_64 -m 1024 -hda disk.img
    /// 
    ///   vex save -d "Ubuntu VM" ubuntu-vm qemu-system-x86_64 -m 2048 -hda ubuntu.qcow2
    /// 
    ///   vex save -y existing-config qemu-system-x86_64 -m 512 -cdrom install.iso
    Save {
        /// Force overwrite existing configuration without prompting
        /// 
        /// Use this flag to automatically overwrite an existing configuration
        /// with the same name without asking for confirmation.
        #[arg(short = 'y', long = "yes")]
        force: bool,

        /// Configuration name for later reference
        /// 
        /// This name will be used to identify the configuration when executing,
        /// listing, renaming, or deleting it. Choose a descriptive name.
        name: String,

        /// Optional description for the configuration
        /// 
        /// Provide a human-readable description of what this configuration does.
        /// Use quotes if the description contains spaces.
        /// Example: -d "Ubuntu 20.04 development environment"
        #[arg(short = 'd', long = "desc")]
        desc: Option<String>,

        /// Path to the QEMU executable
        /// 
        /// Specify the full path or command name for the QEMU binary.
        /// Common examples: qemu-system-x86_64, qemu-system-aarch64, qemu-system-i386
        qemu_bin: String,

        /// QEMU startup arguments
        /// 
        /// All remaining arguments will be passed directly to QEMU when executing
        /// this configuration. You can use any valid QEMU options here.
        /// Examples: -m 1024, -hda disk.img, -cdrom install.iso, -netdev user,id=net0
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        qemu_args: Vec<String>,
    },

    /// Execute a saved QEMU configuration
    /// 
    /// Format: vex exec [OPTIONS] <NAME>
    /// 
    /// This command runs a previously saved QEMU configuration.
    /// It will start QEMU with the saved binary path and arguments.
    /// 
    /// Examples:
    ///   vex exec my-linux
    ///   vex exec -d my-linux    # Start with GDB debugging enabled
    Exec {
        /// Name of the saved configuration to execute
        /// 
        /// This should match a configuration name that was previously saved
        /// using the 'vex save' command. Use 'vex list' to see all available configurations.
        name: String,

        /// Enable GDB debugging mode
        /// 
        /// When enabled, adds '-s -S' parameters to the QEMU command.
        /// This starts a GDB server on localhost:1234 and pauses execution
        /// until a debugger connects. Useful for kernel/OS development.
        #[arg(short = 'd', long = "debug")]
        debug: bool,
    },

    /// Delete a saved QEMU configuration
    /// 
    /// Format: vex rm <NAME>
    /// 
    /// This command permanently removes a saved configuration.
    /// The configuration file will be deleted and cannot be recovered.
    /// 
    /// Examples:
    ///   vex rm old-config
    ///   vex rm my-test-vm
    Rm {
        /// Name of the configuration to delete
        /// 
        /// This should match an existing configuration name.
        /// Use 'vex list' to see all available configurations.
        /// The configuration will be permanently removed.
        name: String,
    },

    /// List all saved configurations
    /// 
    /// Format: vex list
    /// 
    /// This command displays all saved QEMU configurations with their
    /// descriptions, QEMU binary paths, and startup arguments.
    /// Use this to see what configurations are available for execution.
    /// 
    /// Example:
    ///   vex list
    List,

    /// Rename a saved QEMU configuration
    /// 
    /// Format: vex rename [OPTIONS] <OLD_NAME> <NEW_NAME>
    /// 
    /// This command renames an existing configuration and optionally
    /// updates its description. The configuration content remains unchanged.
    /// 
    /// Examples:
    ///   vex rename old-vm new-vm
    ///   vex rename -d "Updated description" my-vm my-new-vm
    ///   vex rename -y old-vm existing-vm    # Force overwrite existing-vm
    Rename {
        /// Update the configuration description
        /// 
        /// Provide a new description for the renamed configuration.
        /// Use quotes if the description contains spaces.
        /// Example: -d "Production Ubuntu server"
        #[arg(short = 'd', long = "desc")]
        desc: Option<String>,

        /// Force overwrite if new name already exists
        /// 
        /// Use this flag to automatically overwrite a configuration
        /// with the new name if it already exists, without prompting.
        #[arg(short = 'y', long = "yes")]
        force: bool,

        /// Current name of the configuration to rename
        /// 
        /// This should match an existing configuration name.
        /// Use 'vex list' to see all available configurations.
        old_name: String,

        /// New name for the configuration
        /// 
        /// The configuration will be renamed to this name.
        /// If a configuration with this name already exists,
        /// you'll be prompted to confirm unless -y flag is used.
        new_name: String,
    },
}

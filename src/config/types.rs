use serde::{Deserialize, Serialize};

/// Stored QEMU configuration structure
#[derive(Debug, Serialize, Deserialize)]
pub struct QemuConfig {
    /// Path to QEMU executable
    pub qemu_bin: String,
    /// List of QEMU startup arguments
    pub args: Vec<String>,
    /// Configuration description (optional)
    pub desc: Option<String>,
}

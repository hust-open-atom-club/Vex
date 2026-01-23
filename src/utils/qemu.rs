use regex::Regex;
use std::process::Command;

/// Get QEMU version string (e.g., "8.2.0")
pub fn get_qemu_version(binary: &str) -> Option<String> {
    let output = Command::new(binary).arg("--version").output().ok()?;

    if !output.status.success() {
        return None;
    }

    let output_str = String::from_utf8_lossy(&output.stdout);

    // QEMU output example: "QEMU emulator version 8.2.0 (v8.2.0)"
    // We capture the first "number.number.number" pattern
    let re = Regex::new(r"version (\d+\.\d+\.\d+)").ok()?;

    re.captures(&output_str).map(|caps| caps[1].to_string())
}

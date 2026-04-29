use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum VexError {
    #[error("Configuration '{name}' not found")]
    ConfigNotFound { name: String },

    #[error("Configuration '{name}' already exists")]
    ConfigAlreadyExists { name: String },

    #[error("Failed to parse configuration")]
    ConfigParseFailed {
        #[source]
        source: serde_json::Error,
    },

    #[error("Failed to serialize configuration")]
    ConfigSerializeFailed {
        #[source]
        source: serde_json::Error,
    },

    #[error("{}", match .field {
        Some(f) => format!("Validation failed on '{}': {}", f, .reason),
        None => format!("Validation failed: {}", .reason),
    })]
    ValidationError {
        field: Option<String>,
        reason: String,
    },

    #[error("IO error on {path}: {operation}")]
    IoError {
        path: PathBuf,
        operation: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to launch QEMU: {binary}")]
    QemuLaunchFailed {
        binary: String,
        #[source]
        source: std::io::Error,
    },

    #[error("QEMU '{binary}' exited with code {}", .exit_code.map_or_else(|| "unknown".to_string(), |c| c.to_string()))]
    QemuExitError {
        binary: String,
        exit_code: Option<i32>,
    },

    #[error("Remote registry not configured (set {env_var})")]
    RemoteNotConfigured { env_var: String },

    #[error("Invalid remote spec '{input}': {reason}")]
    RemoteSpecInvalid { input: String, reason: String },

    #[error("git {args} failed (exit {}){}", .exit_code.map_or_else(|| "unknown".to_string(), |c| c.to_string()), if .stderr.is_empty() { String::new() } else { format!(": {}", .stderr) })]
    GitCommandFailed {
        args: String,
        stderr: String,
        stdout: String,
        exit_code: Option<i32>,
    },

    #[error("Remote config '{id}/{name}:{tag}' not found")]
    RemoteConfigNotFound {
        id: String,
        name: String,
        tag: String,
    },

    #[error("Editor '{editor}' failed (exit {})", .exit_code.map_or_else(|| "unknown".to_string(), |c| c.to_string()))]
    EditorFailed {
        editor: String,
        exit_code: Option<i32>,
    },
}

pub type VexResult<T> = Result<T, VexError>;

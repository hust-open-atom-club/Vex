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

    #[error("git {args} failed (exit {}){}{}", .exit_code.map_or_else(|| "unknown".to_string(), |c| c.to_string()), if .stderr.is_empty() { String::new() } else { format!(": {}", .stderr) }, if .stdout.is_empty() { String::new() } else { format!(" ({})", .stdout) })]
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

    #[error("Unknown resource reference '${{res:{key}}}' in args[{arg_index}]")]
    UnknownResourceReference { key: String, arg_index: usize },

    #[error("Resource '{key}' file not found: {}", .path.display())]
    ResourceFileNotFound { key: String, path: PathBuf },

    #[error("Resource '{key}' checksum mismatch: expected {expected}, got {actual}")]
    ResourceChecksumMismatch {
        key: String,
        expected: String,
        actual: String,
    },

    #[error("Resource '{key}' cannot be published: {reason}")]
    ResourceNotPublishable { key: String, reason: String },

    #[error("Failed to fetch resource from '{url}': {reason}")]
    ResourceFetchFailed { url: String, reason: String },

    #[error("Resource scheme '{scheme}' is not supported (only http/https are downloadable)")]
    UnsupportedResourceScheme { scheme: String },

    #[error("Unsupported published config schema version: {version}")]
    SchemaVersionUnsupported { version: u32 },

    #[error("Hub request to '{url}' failed (status {status}): {body}")]
    HubRequestFailed {
        url: String,
        status: u16,
        body: String,
    },

    #[error("Failed to parse Hub index")]
    HubIndexParseFailed {
        #[source]
        source: serde_json::Error,
    },

    #[error("Hub entry '{id}/{name}:{tag}' not found")]
    HubEntryNotFound {
        id: String,
        name: String,
        tag: String,
    },
}

impl From<std::io::Error> for VexError {
    fn from(err: std::io::Error) -> Self {
        VexError::IoError {
            path: PathBuf::from("<unknown>"),
            operation: "io operation".to_string(),
            source: err,
        }
    }
}

impl From<serde_json::Error> for VexError {
    fn from(err: serde_json::Error) -> Self {
        VexError::ConfigParseFailed { source: err }
    }
}

pub type VexResult<T> = Result<T, VexError>;

use std::fmt;

/// Custom error types for Vex application
#[derive(Debug)]
pub enum VexError {
    ConfigNotFound(String),
    ConfigAlreadyExists(String),
    InvalidConfig(String),
    IoError(std::io::Error),
    SerializationError(serde_json::Error),
}

impl fmt::Display for VexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VexError::ConfigNotFound(name) => write!(f, "Configuration '{}' not found", name),
            VexError::ConfigAlreadyExists(name) => write!(f, "Configuration '{}' already exists", name),
            VexError::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            VexError::IoError(err) => write!(f, "IO error: {}", err),
            VexError::SerializationError(err) => write!(f, "Serialization error: {}", err),
        }
    }
}

impl std::error::Error for VexError {}

impl From<std::io::Error> for VexError {
    fn from(err: std::io::Error) -> Self {
        VexError::IoError(err)
    }
}

impl From<serde_json::Error> for VexError {
    fn from(err: serde_json::Error) -> Self {
        VexError::SerializationError(err)
    }
}
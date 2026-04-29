use crate::config::QemuConfig;
use crate::error::{VexError, VexResult};

pub fn validate_config(config: &QemuConfig) -> VexResult<()> {
    if config.qemu_bin.is_empty() || config.qemu_bin.trim().is_empty() {
        return Err(VexError::ValidationError {
            field: Some("qemu_bin".to_string()),
            reason: "QEMU binary path cannot be empty or whitespace-only".to_string(),
        });
    }

    for (i, arg) in config.args.iter().enumerate() {
        if arg.is_empty() || arg.trim().is_empty() {
            return Err(VexError::ValidationError {
                field: Some(format!("args[{}]", i)),
                reason: "argument cannot be empty or whitespace-only".to_string(),
            });
        }
        if arg.contains('\0') {
            return Err(VexError::ValidationError {
                field: Some(format!("args[{}]", i)),
                reason: "argument cannot contain null bytes".to_string(),
            });
        }
    }

    Ok(())
}

pub fn validate_config_name(name: &str) -> VexResult<()> {
    if name.is_empty() {
        return Err(VexError::ValidationError {
            field: Some("name".to_string()),
            reason: "configuration name cannot be empty".to_string(),
        });
    }

    if name == "." || name == ".." {
        return Err(VexError::ValidationError {
            field: Some("name".to_string()),
            reason: "configuration name cannot be '.' or '..'".to_string(),
        });
    }

    if name.len() > 255 {
        return Err(VexError::ValidationError {
            field: Some("name".to_string()),
            reason: "configuration name exceeds 255 characters".to_string(),
        });
    }

    if name.contains('/') || name.contains('\\') {
        return Err(VexError::ValidationError {
            field: Some("name".to_string()),
            reason: "configuration name cannot contain path separators".to_string(),
        });
    }

    if name.contains('\0') {
        return Err(VexError::ValidationError {
            field: Some("name".to_string()),
            reason: "configuration name cannot contain null bytes".to_string(),
        });
    }

    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    {
        return Err(VexError::ValidationError {
            field: Some("name".to_string()),
            reason: "configuration name may only contain letters, digits, '.', '-', or '_'"
                .to_string(),
        });
    }

    Ok(())
}

pub fn parse_config_json(content: &str) -> VexResult<QemuConfig> {
    serde_json::from_str(content).map_err(|e| VexError::ConfigParseFailed { source: e })
}

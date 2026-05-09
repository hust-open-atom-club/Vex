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

    for (key, resource) in &config.resources {
        validate_resource_key(key)?;
        validate_resource_path(key, &resource.path)?;
        if let Some(sha256) = &resource.sha256 {
            validate_resource_sha256(key, sha256)?;
        }
        if let Some(url) = &resource.url {
            validate_resource_url(key, url)?;
        }
    }

    Ok(())
}

fn validate_resource_url(key: &str, url: &str) -> VexResult<()> {
    if url.contains('\0') {
        return Err(VexError::ValidationError {
            field: Some(format!("resources[{}].url", key)),
            reason: "resource url cannot contain null bytes".to_string(),
        });
    }
    let scheme = if let Some((s, rest)) = url.split_once("://") {
        if rest.is_empty() || rest.starts_with('/') {
            return Err(VexError::ValidationError {
                field: Some(format!("resources[{}].url", key)),
                reason: "resource url must include a host after the scheme".to_string(),
            });
        }
        s
    } else {
        return Err(VexError::ValidationError {
            field: Some(format!("resources[{}].url", key)),
            reason: "resource url must start with http://, https://, or oci://".to_string(),
        });
    };
    if !matches!(scheme, "http" | "https" | "oci") {
        return Err(VexError::ValidationError {
            field: Some(format!("resources[{}].url", key)),
            reason: "resource url must start with http://, https://, or oci://".to_string(),
        });
    }
    Ok(())
}

pub fn validate_resource_key(key: &str) -> VexResult<()> {
    if key.is_empty() {
        return Err(VexError::ValidationError {
            field: Some(format!("resources[{}]", key)),
            reason: "resource key cannot be empty".to_string(),
        });
    }

    let mut chars = key.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(VexError::ValidationError {
            field: Some(format!("resources[{}]", key)),
            reason: "resource key must start with an ASCII letter or underscore".to_string(),
        });
    }

    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return Err(VexError::ValidationError {
                field: Some(format!("resources[{}]", key)),
                reason: "resource key may only contain ASCII letters, digits, or underscore"
                    .to_string(),
            });
        }
    }

    Ok(())
}

fn validate_resource_path(key: &str, path: &str) -> VexResult<()> {
    if path.is_empty() || path.trim().is_empty() {
        return Err(VexError::ValidationError {
            field: Some(format!("resources[{}].path", key)),
            reason: "resource path cannot be empty or whitespace-only".to_string(),
        });
    }
    if path.contains('\0') {
        return Err(VexError::ValidationError {
            field: Some(format!("resources[{}].path", key)),
            reason: "resource path cannot contain null bytes".to_string(),
        });
    }
    Ok(())
}

fn validate_resource_sha256(key: &str, sha256: &str) -> VexResult<()> {
    if sha256.len() != 64 || !sha256.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(VexError::ValidationError {
            field: Some(format!("resources[{}].sha256", key)),
            reason: "resource sha256 must be exactly 64 ASCII hexadecimal characters".to_string(),
        });
    }
    Ok(())
}

pub fn sanitize_config_name(name: &str) -> VexResult<()> {
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

    Ok(())
}

pub fn validate_config_name(name: &str) -> VexResult<()> {
    sanitize_config_name(name)?;

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

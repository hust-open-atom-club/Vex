use std::fs;
use std::path::PathBuf;

use crate::config::{QemuConfig, config_dir, parse_config_json};
use crate::error::{VexError, VexResult};

#[derive(Debug, Clone)]
pub enum ConfigEntry {
    Ok {
        name: String,
        config: QemuConfig,
        path: PathBuf,
    },
    Broken {
        name: String,
        path: PathBuf,
        error: String,
    },
}

impl ConfigEntry {
    pub fn name(&self) -> &str {
        match self {
            ConfigEntry::Ok { name, .. } => name,
            ConfigEntry::Broken { name, .. } => name,
        }
    }
}

pub fn scan_configs() -> VexResult<Vec<ConfigEntry>> {
    let dir = config_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let read = fs::read_dir(&dir).map_err(|e| VexError::IoError {
        path: dir.clone(),
        operation: "read config directory".to_string(),
        source: e,
    })?;

    let mut out: Vec<ConfigEntry> = Vec::new();
    for entry in read {
        let entry = entry.map_err(|e| VexError::IoError {
            path: dir.clone(),
            operation: "read directory entry".to_string(),
            source: e,
        })?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json")
            && let Some(name) = path.file_stem().and_then(|s| s.to_str())
        {
            let name = name.to_string();
            match fs::read_to_string(&path) {
                Ok(content) if content.trim().is_empty() => {
                    out.push(ConfigEntry::Broken {
                        name,
                        path,
                        error: "empty file".to_string(),
                    });
                }
                Ok(content) => match parse_config_json(&content) {
                    Ok(config) => out.push(ConfigEntry::Ok { name, config, path }),
                    Err(_) => out.push(ConfigEntry::Broken {
                        name,
                        path,
                        error: "parse error".to_string(),
                    }),
                },
                Err(_) => out.push(ConfigEntry::Broken {
                    name,
                    path,
                    error: "io error".to_string(),
                }),
            }
        }
    }

    out.sort_by(|a, b| a.name().cmp(b.name()));
    Ok(out)
}

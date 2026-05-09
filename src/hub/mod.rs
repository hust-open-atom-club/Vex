pub mod fetch;
pub mod types;

use std::env;

use crate::error::{VexError, VexResult};

pub const HUB_URL_ENV: &str = "VEX_HUB_URL";
pub const DEFAULT_HUB_URL: &str = "https://hub.vex.example/";

pub fn hub_base_url() -> String {
    env::var(HUB_URL_ENV)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_HUB_URL.to_string())
}

/// Join base URL + path, ensuring exactly one '/' between them.
pub fn join_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    format!("{}/{}", base, path)
}

/// Parse a hub spec of the form `<id>/<name>[:<tag>]`.
///
/// Validation rules mirror those of the Git-backed `RemoteSpec` segment
/// validator, but the implementation is intentionally duplicated here to keep
/// the hub module from importing internals of `crate::remote`.
pub fn parse_hub_spec(s: &str) -> VexResult<(String, String, Option<String>)> {
    let (id, remainder) = s.split_once('/').ok_or_else(|| VexError::ValidationError {
        field: Some("hub_spec".into()),
        reason: "must be in the form <id/name>[:tag]".into(),
    })?;
    let (name, tag) = match remainder.split_once(':') {
        Some((n, t)) => (n, Some(t)),
        None => (remainder, None),
    };
    validate_hub_segment("id", id)?;
    validate_hub_segment("name", name)?;
    if let Some(t) = tag {
        validate_hub_segment("tag", t)?;
    }
    Ok((id.to_string(), name.to_string(), tag.map(String::from)))
}

fn validate_hub_segment(label: &str, value: &str) -> VexResult<()> {
    if value.is_empty() {
        return Err(VexError::ValidationError {
            field: Some(format!("hub_spec.{}", label)),
            reason: format!("{} cannot be empty", label),
        });
    }
    if value.contains('\0') {
        return Err(VexError::ValidationError {
            field: Some(format!("hub_spec.{}", label)),
            reason: format!("{} cannot contain null bytes", label),
        });
    }
    if value.len() > 255 {
        return Err(VexError::ValidationError {
            field: Some(format!("hub_spec.{}", label)),
            reason: format!("{} exceeds 255 characters", label),
        });
    }
    if matches!(value, "." | "..") {
        return Err(VexError::ValidationError {
            field: Some(format!("hub_spec.{}", label)),
            reason: format!("{} cannot be '.' or '..'", label),
        });
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    {
        return Err(VexError::ValidationError {
            field: Some(format!("hub_spec.{}", label)),
            reason: format!(
                "{} '{}' contains unsupported characters; allowed: letters, digits, '.', '-', '_'",
                label, value
            ),
        });
    }
    Ok(())
}

use crate::hub::fetch::fetch_index;

/// Resolve the effective tag for a hub spec.
///
/// If `tag_opt` is `Some`, return it as-is (explicit user choice).
/// If `tag_opt` is `None`, fetch the hub index and look up the entry's
/// `latest_tag` — never assume a `latest.json` alias exists, since the
/// hub protocol does not require it.
pub fn resolve_tag(
    base_url: &str,
    id: &str,
    name: &str,
    tag_opt: Option<&str>,
) -> VexResult<String> {
    if let Some(tag) = tag_opt {
        return Ok(tag.to_string());
    }
    let index = fetch_index(base_url)?;
    index
        .entries
        .iter()
        .find(|e| e.id == id && e.name == name)
        .map(|e| e.latest_tag.clone())
        .ok_or_else(|| VexError::HubEntryNotFound {
            id: id.to_string(),
            name: name.to_string(),
            tag: "latest".to_string(),
        })
}

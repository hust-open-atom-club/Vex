use crate::error::{VexError, VexResult};

const MAX_SNIPPET_NAME_LEN: usize = 64;

/// Validate a snippet name. Snippet names are JSON fields inside
/// `snippets.json`, never used as filenames, so the rules are
/// deliberately more permissive than `validate_config_name`:
///
/// - non-empty after `trim`
/// - at most 64 characters
/// - no `char::is_control()` characters (covers `\n` `\r` `\t` `\0`)
///
/// Spaces, punctuation, and non-ASCII letters are all allowed — that
/// is precisely what makes overrides of builtins like `"1G memory"`
/// possible.
pub fn validate_snippet_name(name: &str) -> VexResult<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(VexError::ValidationError {
            field: Some("name".to_string()),
            reason: "snippet name cannot be empty".to_string(),
        });
    }
    if trimmed.chars().count() > MAX_SNIPPET_NAME_LEN {
        return Err(VexError::ValidationError {
            field: Some("name".to_string()),
            reason: format!("snippet name too long (max {})", MAX_SNIPPET_NAME_LEN),
        });
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(VexError::ValidationError {
            field: Some("name".to_string()),
            reason: "snippet name cannot contain control characters".to_string(),
        });
    }
    Ok(())
}

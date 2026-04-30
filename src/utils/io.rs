use std::io;
use std::path::PathBuf;

use crate::error::{VexError, VexResult};

pub fn prompt_user() -> VexResult<bool> {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| VexError::IoError {
            path: PathBuf::from("stdin"),
            operation: "read user input".to_string(),
            source: e,
        })?;
    let input = input.trim().to_lowercase();
    Ok(input.is_empty() || input == "y" || input == "yes")
}

pub fn prompt_user_default_no() -> VexResult<bool> {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| VexError::IoError {
            path: PathBuf::from("stdin"),
            operation: "read user input".to_string(),
            source: e,
        })?;
    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

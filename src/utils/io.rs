use anyhow::Result;
use std::io;

/// Prompt user for yes/no input, returns true for yes, false for no
/// Default behavior: empty input or 'y'/'yes' returns true for first case,
/// empty input returns false for second case (like overwrite prompts)
pub fn prompt_user() -> Result<bool> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    // For debug parameter prompt: default is yes (Y/n)
    // For overwrite prompt: default is no (y/N)
    // This function handles the "Y/n" case (default yes)
    Ok(input.is_empty() || input == "y" || input == "yes")
}

/// Prompt user for yes/no input with default no (for overwrite prompts)
pub fn prompt_user_default_no() -> Result<bool> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    // For overwrite prompts: only explicit "y" or "yes" returns true
    Ok(input == "y" || input == "yes")
}

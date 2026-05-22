use serde::{Deserialize, Serialize};

/// A single snippet — a named, categorised piece of QEMU args.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snippet {
    pub name: String,
    pub args: Vec<String>,
    pub category: SnippetCategory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnippetCategory {
    Memory,
    Cpu,
    Machine,
    Storage,
    Network,
    Display,
    Debug,
    Kernel,
}

/// On-disk schema for the user snippets file (`snippets.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetFile {
    pub schema_version: u32,
    #[serde(default)]
    pub snippets: Vec<Snippet>,
}

impl SnippetFile {
    pub const CURRENT_VERSION: u32 = 1;
}

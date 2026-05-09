use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HubEntryKind {
    Demo,
    Board,
    Firmware,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubEntry {
    pub id: String,
    pub name: String,
    pub latest_tag: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub summary: String,
    pub kind: HubEntryKind,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubIndex {
    pub schema_version: u32,
    pub entries: Vec<HubEntry>,
}

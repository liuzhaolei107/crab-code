use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub api_provider: Option<String>,
    pub api_base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub small_model: Option<String>,
    pub permission_mode: Option<String>,
    pub system_prompt: Option<String>,
    pub mcp_servers: Option<serde_json::Value>,
    pub hooks: Option<serde_json::Value>,
    pub theme: Option<String>,
}

pub fn load_merged_settings(_project_dir: Option<&PathBuf>) -> crab_common::Result<Settings> {
    // TODO: implement three-level merge (global → user → project)
    Ok(Settings::default())
}

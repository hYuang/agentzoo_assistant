use serde::{Deserialize, Serialize};

/// The plugin API version the host implements. A plugin declares the version
/// it was built against; the host rejects incompatible plugins at load time.
pub const WORKFLOW_API_VERSION: u32 = 1;

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_api_version() -> u32 {
    WORKFLOW_API_VERSION
}

/// The static description of a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManifest {
    /// Stable, URL-safe identifier (e.g. `audiobook`).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default = "default_api_version")]
    pub api_version: u32,
    #[serde(default)]
    pub description: Option<String>,
    /// Capabilities the plugin provides (e.g. `tts`, `audio_post`).
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Host capabilities the plugin needs (e.g. `llm`, `tts`, `file_write`).
    #[serde(default)]
    pub requires: Vec<String>,
}

/// Whether `value` is safe as a plugin id / directory name.
pub fn is_workflow_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}


//! The assistant app's own HTTP contract.
//!
//! The app owns these types; the framework only provides the shared
//! `ApiResult` envelope and generic services. Nothing here is plugin-specific.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::workflow::{ArtifactSpec, WorkflowSpec};

/// The workflow and artifact catalog the UI renders.
#[derive(Debug, Clone, Serialize)]
pub struct Meta {
    pub name: String,
    pub workflow: WorkflowSpec,
    pub artifacts: Vec<ArtifactSpec>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunArtifact {
    pub id: String,
    pub title: String,
    pub path: String,
    pub status: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// One workflow run. The app's run is not a plugin run; there is no plugin id.
#[derive(Debug, Clone, Serialize)]
pub struct Run {
    pub id: String,
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default)]
    pub completed_stages: Vec<String>,
    pub status: String,
    #[serde(default)]
    pub input: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub artifacts: Vec<RunArtifact>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Runs {
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactContent {
    pub content: String,
    pub encoding: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CreateRun {
    #[serde(default)]
    pub input: Value,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StartRun {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub stage: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RejectRun {
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WriteArtifact {
    pub content: String,
    #[serde(default)]
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UploadFile {
    pub name: String,
    pub data: String,
    #[serde(default)]
    pub dir: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UploadFileResponse {
    /// Path relative to the project root.
    pub path: String,
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateDirectory {
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateDirectoryResponse {
    pub path: String,
}

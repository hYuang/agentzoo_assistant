use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::workflow::contribution::ArtifactSpec;

/// The lifecycle of a plugin run (one production instance).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Created,
    Running,
    WaitingGate,
    Completed,
    Failed,
    Cancelled,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Running => "running",
            Self::WaitingGate => "waiting_gate",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// The review state of one artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactStatus {
    Empty,
    Draft,
    Review,
    Approved,
    Locked,
}

impl ArtifactStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Draft => "draft",
            Self::Review => "review",
            Self::Approved => "approved",
            Self::Locked => "locked",
        }
    }
}

/// One artifact of a run, materialized from the plugin's `ArtifactSpec`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunArtifact {
    pub id: String,
    pub title: String,
    /// Path relative to the plugin's canonical artifact directory.
    pub path: String,
    pub status: ArtifactStatus,
    #[serde(default)]
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl RunArtifact {
    /// Whether this artifact is a directory container instead of a file.
    pub fn is_dir(&self) -> bool {
        self.path.ends_with('/') || self.path.ends_with('\\')
    }
}

/// A production instance of a plugin: the persisted state every later stage
/// and UI reads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: String,
    pub plugin_id: String,
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    /// The stage currently executing (or the stage that is waiting at a gate).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    /// The session that executes the run's stages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default)]
    pub completed_stages: Vec<String>,
    pub status: RunStatus,
    #[serde(default)]
    pub input: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Human feedback from the last reject; cleared on the next generate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub artifacts: Vec<RunArtifact>,
}

impl WorkflowRun {
    /// Create a new run, materializing every artifact declared by the plugin.
    pub fn new(
        id: impl Into<String>,
        plugin_id: impl Into<String>,
        project_id: impl Into<String>,
        entry_id: Option<String>,
        input: Value,
        artifacts: &[ArtifactSpec],
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: id.into(),
            plugin_id: plugin_id.into(),
            project_id: project_id.into(),
            entry_id,
            workflow_id: None,
            stage: None,
            session_id: None,
            completed_stages: Vec::new(),
            status: RunStatus::Created,
            input,
            error: None,
            review_notes: None,
            created_at: now.clone(),
            updated_at: now,
            artifacts: artifacts
                .iter()
                .map(|spec| RunArtifact {
                    id: spec.id.clone(),
                    title: spec.title.clone(),
                    path: spec.path.clone().unwrap_or_else(|| spec.id.clone()),
                    status: ArtifactStatus::Empty,
                    size: 0,
                    updated_at: None,
                })
                .collect(),
        }
    }

    pub fn artifact(&self, id: &str) -> Option<&RunArtifact> {
        self.artifacts.iter().find(|artifact| artifact.id == id)
    }

    pub fn artifact_mut(&mut self, id: &str) -> Option<&mut RunArtifact> {
        self.artifacts.iter_mut().find(|artifact| artifact.id == id)
    }

    /// Mark an artifact as written and record its size.
    pub fn touch_artifact(&mut self, id: &str, size: u64) {
        let now = chrono::Utc::now().to_rfc3339();
        if let Some(artifact) = self.artifact_mut(id) {
            artifact.status = ArtifactStatus::Draft;
            artifact.size = size;
            artifact.updated_at = Some(now.clone());
        }
        self.updated_at = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn materializes_artifacts_and_touches_one() {
        let specs = vec![
            ArtifactSpec {
                id: "project".into(),
                title: "Project".into(),
                path: Some("project.json".into()),
                schema: None,
                version: 1,
            },
            ArtifactSpec {
                id: "audio".into(),
                title: "Audio".into(),
                path: Some("audio/".into()),
                schema: None,
                version: 1,
            },
        ];
        let mut run = WorkflowRun::new("run-1", "audiobook", "p-1", None, json!({}), &specs);
        assert_eq!(run.artifacts.len(), 2);
        assert_eq!(
            run.artifact("project").unwrap().status,
            ArtifactStatus::Empty
        );
        assert!(run.artifact("audio").unwrap().is_dir());
        run.touch_artifact("project", 12);
        let project = run.artifact("project").unwrap();
        assert_eq!(project.status, ArtifactStatus::Draft);
        assert_eq!(project.size, 12);
        assert!(project.updated_at.is_some());
    }
}

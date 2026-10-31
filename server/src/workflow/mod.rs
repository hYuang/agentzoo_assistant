//! Workflow/app contract used by this template app.

pub mod contribution;
pub mod manifest;
pub mod run;

#[cfg(test)]
mod tests;

pub use contribution::{
    ArtifactSpec, ChatTaskSpec, GateSpec, WorkflowEntry, WorkflowUiPage, RoleKind, RoleSpec, StageSpec,
    WorkflowSpec,
};
pub use manifest::{is_workflow_id, WorkflowManifest, WORKFLOW_API_VERSION};
pub use run::{ArtifactStatus, WorkflowRun, RunArtifact, RunStatus};

/// The data an app contributes to the host: manifest, workflows, artifacts,
/// UI pages and chat suggestions. Tool registration is separate (the host
/// extension API), so the app does not need a lifecycle/registry abstraction.
pub trait WorkflowApp: Send + Sync {
    fn manifest(&self) -> &WorkflowManifest;

    fn artifact_types(&self) -> Vec<ArtifactSpec> {
        Vec::new()
    }

    fn workflows(&self) -> Vec<WorkflowSpec> {
        Vec::new()
    }

    fn entries(&self) -> Vec<WorkflowEntry> {
        Vec::new()
    }

    fn ui_pages(&self) -> Vec<WorkflowUiPage> {
        Vec::new()
    }

    fn chat_tasks(&self) -> Vec<ChatTaskSpec> {
        Vec::new()
    }
}

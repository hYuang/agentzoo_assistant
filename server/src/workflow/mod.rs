//! The app's workflow model: artifacts, roles, gates, stages, and run state.

pub mod contribution;
pub mod run;

#[cfg(test)]
mod tests;

pub use contribution::{ArtifactSpec, GateSpec, RoleKind, RoleSpec, StageSpec, WorkflowSpec};
pub use run::{ArtifactStatus, RunArtifact, RunStatus, WorkflowRun};

/// The workflow data the app contributes to its own UI and runner.
pub trait WorkflowApp: Send + Sync {
    fn artifact_types(&self) -> Vec<ArtifactSpec> {
        Vec::new()
    }

    fn workflows(&self) -> Vec<WorkflowSpec> {
        Vec::new()
    }
}

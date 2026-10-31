//! Workflow/app contract used by this template app.

pub mod contribution;
pub mod manifest;
pub mod run;

pub use contribution::{
    ArtifactSpec, ChatTaskSpec, GateSpec, PluginEntry, PluginUiPage, RoleKind, RoleSpec, StageSpec,
    WorkflowSpec,
};
pub use manifest::{is_plugin_id, PluginManifest, PLUGIN_API_VERSION};
pub use run::{ArtifactStatus, PluginRun, RunArtifact, RunInit, RunStatus};

/// The data an app contributes to the host: manifest, workflows, artifacts,
/// UI pages and chat suggestions. Tool registration is separate (the host
/// extension API), so the app does not need a lifecycle/registry abstraction.
pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &PluginManifest;

    fn artifact_types(&self) -> Vec<ArtifactSpec> {
        Vec::new()
    }

    fn workflows(&self) -> Vec<WorkflowSpec> {
        Vec::new()
    }

    fn entries(&self) -> Vec<PluginEntry> {
        Vec::new()
    }

    fn ui_pages(&self) -> Vec<PluginUiPage> {
        Vec::new()
    }

    fn ui_asset(&self, _path: &str) -> Option<&'static [u8]> {
        None
    }

    fn chat_tasks(&self) -> Vec<ChatTaskSpec> {
        Vec::new()
    }

    fn init_run(&self, _init: &RunInit) -> Result<(), String> {
        Ok(())
    }
}

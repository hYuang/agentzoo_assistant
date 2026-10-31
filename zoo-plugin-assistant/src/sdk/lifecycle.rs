use async_trait::async_trait;
use zoo_agent::agent::AgentRoute;

use crate::sdk::context::PluginCtx;
use crate::sdk::contribution::{
    ArtifactSpec, ChatTaskSpec, PluginEntry, PluginUiPage, WorkflowSpec,
};
use crate::sdk::error::PluginError;
use crate::sdk::manifest::PluginManifest;
use crate::sdk::PluginTool;
use crate::sdk::run::RunInit;

/// A vertical product plugged into the host.
///
/// A plugin owns its domain model and UI. The host owns the generic lifecycle,
/// storage, scheduling, approvals, and UI shell. The trait deliberately has no
/// agent registration: execution is sourced from the host's existing
/// capability pool.
#[async_trait]
pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &PluginManifest;

    /// Artifact types the plugin owns (schemas, paths, versions).
    fn artifact_types(&self) -> Vec<ArtifactSpec> {
        Vec::new()
    }

    /// Production workflows (workstreams, gates, roles).
    fn workflows(&self) -> Vec<WorkflowSpec> {
        Vec::new()
    }

    /// User-facing entry points (input/output contracts).
    fn entries(&self) -> Vec<PluginEntry> {
        Vec::new()
    }

    /// Pages served from `<plugin>/ui/`.
    fn ui_pages(&self) -> Vec<PluginUiPage> {
        Vec::new()
    }

    /// Domain tools the plugin contributes, each targeted at a specific host
    /// agent by name. The host registers them into the named agent's live tool
    /// registry after the plugin boots, so the agent's LLM can call them from
    /// the next turn onward. A tool whose `agent` does not match a registered
    /// agent is skipped by the host. Empty by default.
    fn tools(&self, _ctx: &PluginCtx) -> Vec<PluginTool> {
        Vec::new()
    }

    /// Preset chat tasks offered in the host's conversation UI (the empty
    /// state of the chat page). The host merges the tasks of every running
    /// plugin; each is rendered as a suggestion button that sends its prompt
    /// to the session's agent.
    fn chat_tasks(&self) -> Vec<ChatTaskSpec> {
        Vec::new()
    }

    /// Deterministic HTTP endpoints owned by the plugin. The host mounts these
    /// at startup using the same project-aware plumbing as agent routes: every
    /// path must include `{pid}`, and the handler receives that project's root.
    fn routes(&self) -> Vec<AgentRoute> {
        Vec::new()
    }

    /// A static UI asset shipped by the plugin, served by the host under
    /// `/plugins/{id}/ui/`. Returning `None` means the plugin has no such
    /// asset. The host only serves the plugin's own bytes; it never renders
    /// vertical UI itself.
    fn ui_asset(&self, _path: &str) -> Option<&'static [u8]> {
        None
    }

    /// Check host compatibility and required capabilities.
    async fn validate(&self, _ctx: &PluginCtx) -> Result<(), PluginError> {
        Ok(())
    }

    /// Create the plugin's data directories and initial state.
    async fn install(&self, _ctx: &PluginCtx) -> Result<(), PluginError> {
        Ok(())
    }

    /// Register contributions and build in-memory state.
    async fn load(&self, _ctx: &PluginCtx) -> Result<(), PluginError> {
        Ok(())
    }

    /// Start background work (providers, watchers, queues).
    async fn start(&self, _ctx: &PluginCtx) -> Result<(), PluginError> {
        Ok(())
    }

    /// Stop background work and flush state.
    async fn stop(&self, _ctx: &PluginCtx) -> Result<(), PluginError> {
        Ok(())
    }

    /// Remove the plugin's data after an explicit uninstall.
    async fn uninstall(&self, _ctx: &PluginCtx) -> Result<(), PluginError> {
        Ok(())
    }

    /// Seed the starting artifacts of a newly created run. The host has
    /// already created the run directory and `run.json`; the plugin writes
    /// only its own artifact files.
    fn init_run(&self, _init: &RunInit) -> Result<(), PluginError> {
        Ok(())
    }
}

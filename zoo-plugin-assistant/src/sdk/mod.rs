//! Local copy of the workflow/app contract used by this template app.
//!
//! The framework no longer owns a plugin registry; these types are private to
//! the assistant app so its workflow/run engine stays business-local.

pub mod context;
pub mod contribution;
pub mod error;
pub mod lifecycle;
pub mod manifest;
pub mod run;

use std::sync::Arc;

use zoo_agent::tool::Tool;

pub use context::{PluginCtx, PluginRunControl};
pub use contribution::{
    ArtifactSpec, ChatTaskSpec, GateSpec, PluginEntry, PluginUiPage, RoleKind, RoleSpec, StageSpec,
    WorkflowSpec,
};
pub use error::PluginError;
pub use lifecycle::Plugin;
pub use manifest::{is_plugin_id, validate_manifest, PluginManifest, PLUGIN_API_VERSION};
pub use run::{ArtifactStatus, PluginRun, RunArtifact, RunInit, RunStatus};

pub use zoo_agent::tool::{ToolCtx, ToolError};

/// A tool contributed to one host agent.
pub struct PluginTool {
    pub agent: String,
    pub tool: Arc<dyn Tool>,
}

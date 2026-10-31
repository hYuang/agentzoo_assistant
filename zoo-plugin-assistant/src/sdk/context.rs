use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::mpsc;
use zoo_agent::agent::{AgentCatalog, SpeechSynthesizer};
use zoo_agent::session::RuntimeHooks;
use zoo_agent::DispatcherRef;
use zoo_shared::event::Envelope;
use zoo_store::{ConversationStore, ReportStore, SessionStore, TaskStore};

use crate::sdk::manifest::PLUGIN_API_VERSION;

/// Host capability that lets a plugin-contributed tool create and start a
/// plugin run. The chat-first plugins use it so a conversational intake can
/// launch the same workflow the dashboard would.
#[async_trait::async_trait]
pub trait PluginRunControl: Send + Sync {
    /// Create a run for one plugin entry and return its run id.
    async fn create_run(
        &self,
        project_id: &str,
        plugin_id: &str,
        entry_id: &str,
        input: Value,
    ) -> Result<String, String>;

    /// Start the next ready stage (or a named stage) for a run.
    async fn start_run(
        &self,
        project_id: &str,
        plugin_id: &str,
        run_id: &str,
        stage: Option<&str>,
    ) -> Result<(), String>;
}

/// The host services a plugin may use during its lifecycle.
///
/// The context is intentionally made of trait objects and paths: a plugin never
/// sees the concrete storage backend, the server state, or another plugin.
/// Per-run values (project root, run id, inputs) are added by the host when it
/// invokes a workflow; they are not part of the long-lived plugin context.
#[derive(Clone)]
pub struct PluginCtx {
    pub plugin_id: String,
    /// `<data_dir>/plugins/<plugin_id>` for a plugin context.
    pub plugin_dir: PathBuf,
    /// The host data directory.
    pub data_dir: PathBuf,
    pub host_api: u32,
    pub tasks: Option<Arc<dyn TaskStore>>,
    pub reports: Option<Arc<dyn ReportStore>>,
    /// The host's conversation and session stores, so a plugin can read the
    /// user's chats (metadata + transcripts) for its own analytics.
    pub conversations: Option<Arc<dyn ConversationStore>>,
    pub sessions: Option<Arc<dyn SessionStore>>,
    pub events: Option<mpsc::Sender<Envelope>>,
    pub dispatcher: Option<DispatcherRef>,
    pub catalog: Option<Arc<dyn AgentCatalog>>,
    pub tts: Option<Arc<dyn SpeechSynthesizer>>,
    /// Create/start plugin runs, so plugin tools can launch a workflow from a
    /// conversation.
    pub runs: Option<Arc<dyn PluginRunControl>>,
    /// The host's per-agent runtimes, keyed by agent name. A plugin can read
    /// an agent's prompt layers, tool specs, and context budget.
    pub runtimes: Option<Arc<std::sync::RwLock<std::collections::HashMap<String, Arc<dyn RuntimeHooks>>>>>,
}

impl PluginCtx {
    /// A host context with no plugin selected yet and no services attached.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        let data_dir = data_dir.into();
        let plugin_dir = data_dir.join("plugins");
        Self {
            plugin_id: String::new(),
            plugin_dir,
            data_dir,
            host_api: PLUGIN_API_VERSION,
            tasks: None,
            reports: None,
            conversations: None,
            sessions: None,
            events: None,
            dispatcher: None,
            catalog: None,
            tts: None,
            runs: None,
            runtimes: None,
        }
    }

    /// Clone this context for one plugin: the plugin directory becomes
    /// `<data_dir>/plugins/<id>`.
    pub fn for_plugin(&self, id: impl Into<String>) -> Self {
        let plugin_id = id.into();
        let plugin_dir = self.data_dir.join("plugins").join(&plugin_id);
        Self {
            plugin_id,
            plugin_dir,
            data_dir: self.data_dir.clone(),
            host_api: self.host_api,
            tasks: self.tasks.clone(),
            reports: self.reports.clone(),
            conversations: self.conversations.clone(),
            sessions: self.sessions.clone(),
            events: self.events.clone(),
            dispatcher: self.dispatcher.clone(),
            catalog: self.catalog.clone(),
            tts: self.tts.clone(),
            runs: self.runs.clone(),
            runtimes: self.runtimes.clone(),
        }
    }

    /// The plugin's install directory.
    pub fn install_dir(&self) -> &Path {
        &self.plugin_dir
    }

    /// The plugin's project-scoped data directory:
    /// `<project>/.zoo_agent/plugins/<id>`.
    pub fn project_dir(&self, project_root: &Path) -> PathBuf {
        project_root
            .join(".zoo_agent")
            .join("plugins")
            .join(&self.plugin_id)
    }

    /// The plugin's runtime state directory:
    /// `<project>/.zoo_agent/plugins/<id>/runs/<run>`.
    pub fn run_dir(&self, project_root: &Path, run: &str) -> PathBuf {
        self.project_dir(project_root).join("runs").join(run)
    }

    /// The plugin's private material root, kept under `.zoo_agent` alongside
    /// the run state: `<project>/.zoo_agent/plugins/<id>/assets`.
    pub fn assets_dir(&self, project_root: &Path) -> PathBuf {
        self.project_dir(project_root).join("assets")
    }

    /// The plugin's upload root:
    /// `<project>/.zoo_agent/plugins/<id>/assets/uploads`.
    pub fn uploads_dir(&self, project_root: &Path) -> PathBuf {
        self.assets_dir(project_root).join("uploads")
    }

    /// The plugin's canonical, Git-trackable workspace:
    /// `<project>/<id>`.
    pub fn artifact_dir(&self, project_root: &Path) -> PathBuf {
        project_root.join(&self.plugin_id)
    }
}

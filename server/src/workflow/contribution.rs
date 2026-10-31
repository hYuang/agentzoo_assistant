use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A persisted artifact type a plugin owns (e.g. `story_bible`, `script`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSpec {
    pub id: String,
    pub title: String,
    /// Relative path below the plugin's run directory.
    #[serde(default)]
    pub path: Option<String>,
    /// JSON schema of the artifact's content, when it is structured.
    #[serde(default)]
    pub schema: Option<Value>,
    #[serde(default = "default_artifact_version")]
    pub version: u32,
}

fn default_artifact_version() -> u32 {
    1
}

/// Who fills a role: a human or an agent from the host's capability pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleKind {
    Human,
    Agent,
}

/// A business role in the plugin's workflow. Roles are not agent names; the
/// host resolves them against its capability catalog at run time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleSpec {
    pub id: String,
    pub name: String,
    #[serde(default = "default_role_kind")]
    pub kind: RoleKind,
    /// Capabilities the executor of this role must provide.
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Artifact ids this role may approve.
    #[serde(default)]
    pub can_approve: Vec<String>,
}

fn default_role_kind() -> RoleKind {
    RoleKind::Agent
}

/// A decision gate between workstreams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateSpec {
    pub id: String,
    pub name: String,
    /// Role id that decides the gate.
    pub approver: String,
    /// Where a rejection returns (a stage id); `None` means the gate's stage.
    #[serde(default)]
    pub on_reject: Option<String>,
}

/// One stage in a workstream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageSpec {
    pub id: String,
    pub name: String,
    /// Role id that owns the stage.
    pub role: String,
    /// Capabilities required to execute the stage.
    #[serde(default)]
    pub requires: Vec<String>,
    /// Stage ids this stage waits for.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Artifact ids the stage reads.
    #[serde(default)]
    pub inputs: Vec<String>,
    /// Artifact ids produced by the stage.
    #[serde(default)]
    pub outputs: Vec<String>,
    /// Gate id that must pass after the stage, if any.
    #[serde(default)]
    pub gate: Option<String>,
    /// Optional fan-out expression (e.g. `script.chapters`).
    #[serde(default)]
    pub foreach: Option<String>,
    /// Stage instruction handed to the executor. Domain-specific; not part of
    /// the public client catalog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

/// A production workflow: roles, gates, and the stage graph they govern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSpec {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub roles: Vec<RoleSpec>,
    #[serde(default)]
    pub gates: Vec<GateSpec>,
    #[serde(default)]
    pub stages: Vec<StageSpec>,
}

/// A user-facing entry point: the input contract and optional custom UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEntry {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    /// JSON schema of the launch input.
    #[serde(default)]
    pub input_schema: Value,
    /// JSON schema of the delivered output.
    #[serde(default)]
    pub output_schema: Value,
    /// UI page under the plugin's `ui/` directory, when the entry has a
    /// custom launcher instead of the generic form renderer.
    #[serde(default)]
    pub ui_page: Option<String>,
}

/// A page the plugin ships for its own UI (served from `/plugins/{id}/ui/`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowUiPage {
    pub id: String,
    pub title: String,
    pub page: String,
    /// Where the host may mount it (e.g. `plugin.dashboard`).
    #[serde(default)]
    pub slot: String,
}

/// A preset chat task a plugin offers in the host's conversation UI. The host
/// renders the task as a suggestion button; clicking it sends `prompt` to the
/// session's agent. Labels and prompts are localized by the plugin (the host
/// picks the variant per locale).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTaskSpec {
    pub id: String,
    pub label_zh: String,
    pub label_en: String,
    pub prompt_zh: String,
    pub prompt_en: String,
}

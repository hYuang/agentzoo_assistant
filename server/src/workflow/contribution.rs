use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A persisted artifact type the app owns (e.g. `materials`, `quiz`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSpec {
    pub id: String,
    pub title: String,
    /// Relative path below the app's run directory.
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

/// A business role in the app's workflow. Roles are not agent names; the
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
    /// Stage instruction handed to the executor. Internal; never sent to the
    /// client catalog.
    #[serde(default, skip_serializing)]
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

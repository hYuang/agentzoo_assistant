//! Observability / BI vertical plugin.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use serde_json::{json, Value};
use zoo_plugins::{
    AgentRoute, ArtifactSpec, ChatTaskSpec, GateSpec, HttpCtx, HttpError, HttpHandler, Plugin,
    PluginCtx, PluginEntry, PluginManifest, PluginUiPage, RoleKind, RoleSpec, StageSpec,
    WorkflowSpec, PLUGIN_API_VERSION,
};

mod analysis;

pub struct ObservabilityPlugin {
    ctx: Arc<PluginCtx>,
}

impl ObservabilityPlugin {
    /// Wrap the plugin with its boot-time context. The registry builds the
    /// unit struct; the host installs the context right after registration,
    /// before any route can be mounted.
    pub fn with_context(ctx: Arc<PluginCtx>) -> Self {
        Self { ctx }
    }
}

impl Plugin for ObservabilityPlugin {
    fn manifest(&self) -> &PluginManifest {
        static M: OnceLock<PluginManifest> = OnceLock::new();
        M.get_or_init(|| PluginManifest {
            id: "observability".into(),
            name: "Observability".into(),
            version: "0.1.0".into(),
            api_version: PLUGIN_API_VERSION,
            description: Some(
                "Session and task analysis: latency, failures, tokens, bottlenecks.".into(),
            ),
            capabilities: vec!["workflow".into(), "artifacts".into(), "analytics".into()],
            requires: vec!["llm".into()],
        })
    }

    fn artifact_types(&self) -> Vec<ArtifactSpec> {
        vec![
            artifact("scope", "Scope", "scope.json"),
            artifact("dataset", "Dataset", "dataset.json"),
            artifact("insight_report", "Insight Report", "report.md"),
        ]
    }

    fn workflows(&self) -> Vec<WorkflowSpec> {
        vec![WorkflowSpec {
            id: "observability_flow".into(),
            name: "Observability analysis".into(),
            roles: vec![
                role("analyst", "Analyst", RoleKind::Agent, vec!["data_analysis", "file_write"], vec![]),
                role("reviewer", "Reviewer", RoleKind::Human, vec![], vec!["report"]),
            ],
            gates: vec![gate("report", "Report approval", "reviewer", Some("report"))],
            stages: vec![
                stage("scope", "Scope", "analyst", vec!["data_analysis"], vec![], vec!["scope"]),
                stage("query", "Query", "analyst", vec!["data_analysis"], vec!["scope"], vec!["dataset"]),
                stage("analyze", "Analyze", "analyst", vec!["data_analysis"], vec!["query"], vec!["dataset"]),
                stage("report", "Report", "analyst", vec!["file_write"], vec!["analyze"], vec!["insight_report"]),
            ],
        }]
    }

    fn entries(&self) -> Vec<PluginEntry> {
        vec![PluginEntry {
            id: "observability".into(),
            title: "Observability".into(),
            description: "Analyze session and task runs for latency, failures, and cost.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "range": { "type": "string", "enum": ["24h", "7d", "30d", "all"] },
                    "group_by": { "type": "string", "enum": ["agent", "model", "tool", "task", "session", "time"] },
                    "focus": { "type": "array", "items": { "type": "string" } }
                }
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "report": { "type": "string" },
                    "dataset": { "type": "string" }
                }
            }),
            ui_page: Some("index.html".into()),
        }]
    }

    fn ui_pages(&self) -> Vec<PluginUiPage> {
        vec![PluginUiPage {
            id: "observability".into(),
            title: "Observability".into(),
            page: "index.html".into(),
            slot: "plugin.dashboard".into(),
        }]
    }

    fn chat_tasks(&self) -> Vec<ChatTaskSpec> {
        vec![
            ChatTaskSpec {
                id: "latency_report".into(),
                label_zh: "延迟与失败分析".into(),
                label_en: "Latency & failure analysis".into(),
                prompt_zh: "请分析本项目最近的会话与任务数据：延迟分布、失败率、token 消耗，指出瓶颈并给出改进建议。".into(),
                prompt_en: "Analyze this project's recent session and task data: latency distribution, failure rate, and token usage. Identify bottlenecks and suggest improvements.".into(),
            },
            ChatTaskSpec {
                id: "usage_trends".into(),
                label_zh: "用量趋势".into(),
                label_en: "Usage trends".into(),
                prompt_zh: "请汇总本项目的 agent 用量趋势：各 agent 的调用次数、成功率和平均耗时，按时间维度给出变化。".into(),
                prompt_en: "Summarize agent usage trends for this project: call counts, success rates, and average duration per agent, with changes over time.".into(),
            },
        ]
    }

    fn routes(&self) -> Vec<AgentRoute> {
        let ctx = Arc::clone(&self.ctx);
        vec![
            AgentRoute {
                method: "GET",
                path: "/projects/{pid}/plugins/observability/analysis/conversations",
                handler: Arc::new(ConversationsHandler { ctx: Arc::clone(&ctx) }),
            },
            AgentRoute {
                method: "GET",
                path: "/projects/{pid}/plugins/observability/analysis/conversations/{cid}",
                handler: Arc::new(ConversationDetailHandler { ctx: Arc::clone(&ctx) }),
            },
            AgentRoute {
                method: "GET",
                path: "/projects/{pid}/plugins/observability/analysis/logs",
                handler: Arc::new(LogsHandler { ctx: Arc::clone(&ctx) }),
            },
            AgentRoute {
                method: "GET",
                path: "/projects/{pid}/plugins/observability/agents",
                handler: Arc::new(AgentsHandler { ctx: Arc::clone(&ctx) }),
            },
            AgentRoute {
                method: "GET",
                path: "/projects/{pid}/plugins/observability/analysis/sessions/{sid}",
                handler: Arc::new(SessionStatsHandler { ctx: Arc::clone(&ctx) }),
            },
        ]
    }

    fn ui_asset(&self, path: &str) -> Option<&'static [u8]> {
        match path {
            "" | "index.html" => Some(include_bytes!("../ui/index.html")),
            _ => None,
        }
    }
}

struct ConversationsHandler {
    ctx: Arc<PluginCtx>,
}

#[async_trait]
impl HttpHandler for ConversationsHandler {
    async fn handle(
        &self,
        http: &HttpCtx,
        _params: &HashMap<String, String>,
        _body: &Value,
    ) -> Result<Value, HttpError> {
        analysis::conversations(&self.ctx, &http.project_id).await
    }
}

struct ConversationDetailHandler {
    ctx: Arc<PluginCtx>,
}

#[async_trait]
impl HttpHandler for ConversationDetailHandler {
    async fn handle(
        &self,
        http: &HttpCtx,
        params: &HashMap<String, String>,
        _body: &Value,
    ) -> Result<Value, HttpError> {
        let cid = params
            .get("cid")
            .ok_or_else(|| HttpError::InvalidRequest("missing conversation id".into()))?;
        analysis::conversation_detail(&self.ctx, &http.project_id, cid).await
    }
}

struct LogsHandler {
    ctx: Arc<PluginCtx>,
}

#[async_trait]
impl HttpHandler for LogsHandler {
    async fn handle(
        &self,
        _http: &HttpCtx,
        _params: &HashMap<String, String>,
        body: &Value,
    ) -> Result<Value, HttpError> {
        let limit = body
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(500)
            .min(2000) as usize;
        let level = body
            .get("level")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let query = body
            .get("q")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        analysis::logs(&self.ctx, limit, &level, &query).await
    }
}

struct AgentsHandler {
    ctx: Arc<PluginCtx>,
}

#[async_trait]
impl HttpHandler for AgentsHandler {
    async fn handle(
        &self,
        http: &HttpCtx,
        _params: &HashMap<String, String>,
        _body: &Value,
    ) -> Result<Value, HttpError> {
        analysis::agents(&self.ctx, &http.project_id).await
    }
}

struct SessionStatsHandler {
    ctx: Arc<PluginCtx>,
}

#[async_trait]
impl HttpHandler for SessionStatsHandler {
    async fn handle(
        &self,
        http: &HttpCtx,
        params: &HashMap<String, String>,
        _body: &Value,
    ) -> Result<Value, HttpError> {
        let sid = params
            .get("sid")
            .ok_or_else(|| HttpError::InvalidRequest("missing session id".into()))?;
        analysis::session_stats(&self.ctx, &http.project_id, sid).await
    }
}

fn artifact(id: &str, title: &str, path: &str) -> ArtifactSpec {
    ArtifactSpec { id: id.into(), title: title.into(), path: Some(path.into()), schema: None, version: 1 }
}

fn role(id: &str, name: &str, kind: RoleKind, capabilities: Vec<&str>, can_approve: Vec<&str>) -> RoleSpec {
    RoleSpec {
        id: id.into(),
        name: name.into(),
        kind,
        capabilities: capabilities.into_iter().map(str::to_string).collect(),
        can_approve: can_approve.into_iter().map(str::to_string).collect(),
    }
}

fn gate(id: &str, name: &str, approver: &str, on_reject: Option<&str>) -> GateSpec {
    GateSpec { id: id.into(), name: name.into(), approver: approver.into(), on_reject: on_reject.map(str::to_string) }
}

fn stage(id: &str, name: &str, role: &str, requires: Vec<&str>, depends_on: Vec<&str>, outputs: Vec<&str>) -> StageSpec {
    StageSpec {
        id: id.into(),
        name: name.into(),
        role: role.into(),
        requires: requires.into_iter().map(str::to_string).collect(),
        depends_on: depends_on.into_iter().map(str::to_string).collect(),
        inputs: vec![],
        outputs: outputs.into_iter().map(str::to_string).collect(),
        gate: None,
        foreach: None,
        prompt: None,
    }
}

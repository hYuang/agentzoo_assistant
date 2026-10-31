//! Learning-assistant vertical plugin.
//!
//! Turns a topic or a set of study materials into a knowledge map, a study
//! plan, notes, practice questions, a review sheet, and a final report. The
//! workflow is intentionally staged: the learner approves the plan and the
//! practice set before the assistant moves on.

use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use zoo_agent::tool::{Tool, ToolCtx, ToolError};
use crate::sdk::{
    ArtifactSpec, ChatTaskSpec, GateSpec, Plugin, PluginCtx, PluginEntry, PluginManifest,
    PluginTool, PluginUiPage, RoleKind, RoleSpec, StageSpec, WorkflowSpec,
    PLUGIN_API_VERSION,
};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use zoo_agent::agent::AgentContext;
use zoo_bootstrap::{AgentTool, AppExtension};
use zoo_server::ServerState;
use zoo_shared::api::{ApiResult, PluginsResponse};
use zoo_shared::api::{CreatePluginRunRequest, StartPluginRunRequest};
use zoo_shared::tool::ToolSpec;

pub mod catalog;
pub mod runs;
pub mod runs_api;
pub mod sdk;
pub mod ui;

pub struct AssistantPlugin;

/// The shared tutor instructions, appended to every stage's specific task.
const TUTOR_PROMPT: &str = include_str!("../prompts/assistant.md");

impl Plugin for AssistantPlugin {
    fn manifest(&self) -> &PluginManifest {
        static M: OnceLock<PluginManifest> = OnceLock::new();
        M.get_or_init(|| PluginManifest {
            id: "assistant".into(),
            name: "My Learning Assistant".into(),
            version: "0.1.0".into(),
            api_version: PLUGIN_API_VERSION,
            description: Some(
                "Learning assistant: turn a topic or materials into a knowledge map, study plan, notes, practice, and review."
                    .into(),
            ),
            capabilities: vec!["workflow".into(), "artifacts".into()],
            requires: vec![
                "llm".into(),
                "file_read".into(),
                "web_search".into(),
                "file_write".into(),
            ],
        })
    }

    fn artifact_types(&self) -> Vec<ArtifactSpec> {
        vec![
            artifact("materials", "Materials", "materials.json"),
            artifact("knowledge", "Knowledge points", "knowledge-map.md"),
            artifact("quiz", "Timed practice", "quiz.json"),
            artifact("learner_state", "Practice progress", "learner-state.json"),
            artifact("review", "Analysis", "review.json"),
            artifact("report", "Report", "report.md"),
        ]
    }

    fn workflows(&self) -> Vec<WorkflowSpec> {
        vec![WorkflowSpec {
            id: "timed_practice_flow".into(),
            name: "Study a topic".into(),
            roles: vec![
                role(
                    "learner",
                    "Learner",
                    RoleKind::Human,
                    vec![],
                    vec!["practice"],
                ),
                role(
                    "tutor",
                    "Tutor",
                    RoleKind::Agent,
                    vec!["file_read", "web_search", "file_write"],
                    vec![],
                ),
            ],
            gates: vec![gate(
                "practice",
                "Practice review",
                "learner",
                Some("practice"),
            )],
            stages: vec![
                stage(
                    "intake",
                    "Materials",
                    "tutor",
                    vec!["file_read"],
                    vec![],
                    vec!["materials"],
                    "读取输入中的 topic、materials、syllabus、past_papers、level。把资料整理成 materials.json，结构：\
                     {\"items\":[{\"id\":\"m1\",\"title\":\"...\",\"kind\":\"material|knowledge_point|syllabus|past_paper|model\",\"location\":\"...\",\"summary\":\"...\",\"key_points\":[\"...\"]}]}。\
                     把考纲和往年真题也作为 items 收录，题目生成阶段会参考。\
                     如果输入没有指定资料，先用 `kb list` 查看项目知识库，把相关文件列入 materials.json。\
                     不要编造资料；读不到时写明原因。没有外部资料时，用你的知识生成知识点大纲，kind 标为 \"model\"。\
                     如果 topic 和 materials 都为空，就在 items 里写一条需要学习者补充主题或资料的空状态记录，并继续流程。",
                ),
                stage(
                    "map",
                    "Knowledge points",
                    "tutor",
                    vec!["file_read", "web_search", "file_write"],
                    vec!["intake"],
                    vec!["knowledge"],
                    "基于 materials.json 生成 knowledge-map.md：知识点清单、考纲要求、重点/易错点、知识点之间的前置关系，以及题型分布。\
                     如果提供了往年真题，总结题型、难度和考点分布；引用资料 id。",
                ),
                gated(
                    stage(
                        "practice",
                        "Timed practice",
                        "tutor",
                        vec!["file_read", "file_write"],
                        vec!["map"],
                        vec!["quiz"],
                        "基于 knowledge-map.md 和 materials.json（知识点、考纲、往年真题）生成一套限时练习题 quiz.json，结构：\
                         {\"title\":\"...\",\"time_limit_minutes\":30,\"question_count\":20,\"instructions\":\"...\",\"questions\":[\
                         {\"id\":\"q1\",\"type\":\"choice\",\"question\":\"...\",\"options\":[\"...\"],\"answer\":0,\"explanation\":\"...\",\"difficulty\":\"easy|medium|hard\",\"concept\":\"...\",\"source\":\"knowledge_point|syllabus|past_paper\"},\
                         {\"id\":\"q2\",\"type\":\"short\",\"question\":\"...\",\"answer\":\"参考答案\",\"explanation\":\"...\",\"difficulty\":\"medium\",\"concept\":\"...\",\"source\":\"...\"},\
                         {\"id\":\"q3\",\"type\":\"code\",\"question\":\"...\",\"answer\":\"参考代码\",\"explanation\":\"...\",\"difficulty\":\"hard\",\"concept\":\"...\",\"source\":\"...\"}]}。\
                         题量取 run.input 的 question_count（缺省 20），限时取 time_limit_minutes（缺省 30）。\
                         题目要覆盖知识点和考纲，参考往年真题的题型/难度分布；不要逐字照抄真题，引用时在 source 中标注。\
                         choice 的 answer 是正确选项的 0-based 索引；short/code 的 answer 是参考答案。答案与解析必须齐全。",
                    ),
                    "practice",
                ),
                stage(
                    "review",
                    "Analysis",
                    "tutor",
                    vec!["file_read", "file_write"],
                    vec!["practice"],
                    vec!["review"],
                    "读取 quiz.json 与 learner-state.json（答案、自评、开始/提交时间、用时），生成 review.json，结构：\
                     {\"summary\":\"...\",\"score\":8,\"total\":10,\"accuracy\":0.8,\"elapsed_seconds\":600,\"time_limit_minutes\":30,\
                     \"weak_concepts\":[\"...\"],\"mistakes\":[{\"question_id\":\"q1\",\"question\":\"...\",\"your_answer\":\"...\",\"correct_answer\":\"...\",\"cause\":\"...\",\"approach\":\"...\",\"concept\":\"...\"}],\
                     \"cards\":[{\"id\":\"c1\",\"front\":\"...\",\"back\":\"...\",\"concept\":\"...\",\"due\":\"...\"}],\"recommendations\":[\"...\"]}。\
                     分析要基于实际答案和用时：区分知识盲区、审题/粗心、时间不足；如果 learner-state.json 还没有答案，先生成待作答清单，不要臆造答案。",
                ),
                stage(
                    "report",
                    "Report",
                    "tutor",
                    vec!["file_read", "file_write"],
                    vec!["review"],
                    vec!["report"],
                    "汇总 materials、knowledge-map、quiz、learner-state、review 生成 report.md：练习成绩、用时、正确率、薄弱知识点、错因分类、下一步刷题建议。引用资料 id。",
                ),
            ],
        }]
    }

    fn entries(&self) -> Vec<PluginEntry> {
        vec![PluginEntry {
            id: "learning".into(),
            title: "Timed practice session".into(),
            description: "Turn knowledge points, a syllabus, or past papers into a timed practice set, then analyze the result."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "topic": { "type": "string", "description": "Topic or exam to practice" },
                    "materials": { "type": "string", "description": "Knowledge points or study material: project-relative path, URL, or pasted text" },
                    "syllabus": { "type": "string", "description": "Exam syllabus: project-relative path, URL, or pasted text" },
                    "past_papers": { "type": "string", "description": "Past exam papers: project-relative path, URL, or pasted text" },
                    "level": {
                        "type": "string",
                        "enum": ["beginner", "intermediate", "advanced"],
                        "description": "Current level"
                    },
                    "goal": { "type": "string", "description": "Score or skill target" },
                    "question_count": { "type": "integer", "description": "Number of questions (default 20)" },
                    "time_limit_minutes": { "type": "integer", "description": "Timed practice length in minutes (default 30)" }
                },
                "required": ["topic"]
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "materials": { "type": "string" },
                    "knowledge": { "type": "string" },
                    "quiz": { "type": "string" },
                    "learner_state": { "type": "string" },
                    "review": { "type": "string" },
                    "report": { "type": "string" }
                }
            }),
            ui_page: Some("index.html".into()),
        }]
    }

    fn ui_pages(&self) -> Vec<PluginUiPage> {
        vec![PluginUiPage {
            id: "assistant".into(),
            title: "My Learning Assistant".into(),
            page: "index.html".into(),
            slot: "plugin.dashboard".into(),
        }]
    }

    fn ui_asset(&self, path: &str) -> Option<&'static [u8]> {
        match path {
            "" | "index.html" => Some(include_bytes!("../ui/index.html")),
            _ => None,
        }
    }

    fn tools(&self, _ctx: &PluginCtx) -> Vec<PluginTool> {
        Vec::new()
    }

    fn chat_tasks(&self) -> Vec<ChatTaskSpec> {
        vec![
            ChatTaskSpec {
                id: "analyze_syllabus".into(),
                label_zh: "分析考纲与真题".into(),
                label_en: "Analyze syllabus and past papers".into(),
                prompt_zh: "请分析我提供的考纲和往年真题：整理知识点、题型、难度和考点分布，作为出题依据。".into(),
                prompt_en: "Analyze the syllabus and past papers I provided: summarize knowledge points, question types, difficulty, and topic distribution as the basis for a practice set.".into(),
            },
            ChatTaskSpec {
                id: "timed_practice".into(),
                label_zh: "开始限时刷题".into(),
                label_en: "Start timed practice".into(),
                prompt_zh: "请先一次一个问题地询问主题、知识点/考纲/往年真题、题量和限时（能选项的用 ask_user 选项）；确认后调用 start_practice 工具创建并启动限时刷题。".into(),
                prompt_en: "Ask me one question at a time for the topic, knowledge points/syllabus/past papers, question count, and time limit (use ask_user options when the answer is a small set); after confirming, call the start_practice tool to create and start the timed practice.".into(),
            },
            ChatTaskSpec {
                id: "review_mistakes".into(),
                label_zh: "分析刷题结果".into(),
                label_en: "Analyze practice result".into(),
                prompt_zh: "请根据我的答题结果和用时分析这次刷题：正确率、薄弱知识点、错因分类、时间分配和下一步刷题建议。".into(),
                prompt_en: "Analyze this practice run from my answers and time spent: accuracy, weak knowledge points, mistake causes, time allocation, and what to practice next.".into(),
            },
            ChatTaskSpec {
                id: "explain_material".into(),
                label_zh: "讲解知识点".into(),
                label_en: "Explain knowledge points".into(),
                prompt_zh: "请讲解我提供的知识点或错题：给出直觉、例子、易错点和同类题思路。".into(),
                prompt_en: "Explain the knowledge points or mistakes I provided: intuition, examples, common pitfalls, and how to approach similar questions.".into(),
            },
            ChatTaskSpec {
                id: "learning_report".into(),
                label_zh: "刷题报告".into(),
                label_en: "Practice report".into(),
                prompt_zh: "请汇总这次刷题结果，生成一份报告：成绩、用时、正确率、薄弱知识点和下一步刷题计划。".into(),
                prompt_en: "Summarize this practice run into a report: score, time spent, accuracy, weak knowledge points, and the next practice plan.".into(),
            },
        ]
    }
}

fn artifact(id: &str, title: &str, path: &str) -> ArtifactSpec {
    ArtifactSpec {
        id: id.into(),
        title: title.into(),
        path: Some(path.into()),
        schema: None,
        version: 1,
    }
}

fn role(
    id: &str,
    name: &str,
    kind: RoleKind,
    capabilities: Vec<&str>,
    can_approve: Vec<&str>,
) -> RoleSpec {
    RoleSpec {
        id: id.into(),
        name: name.into(),
        kind,
        capabilities: capabilities.into_iter().map(str::to_string).collect(),
        can_approve: can_approve.into_iter().map(str::to_string).collect(),
    }
}

fn gate(id: &str, name: &str, approver: &str, on_reject: Option<&str>) -> GateSpec {
    GateSpec {
        id: id.into(),
        name: name.into(),
        approver: approver.into(),
        on_reject: on_reject.map(str::to_string),
    }
}

fn stage(
    id: &str,
    name: &str,
    role: &str,
    requires: Vec<&str>,
    depends_on: Vec<&str>,
    outputs: Vec<&str>,
    specific_prompt: &str,
) -> StageSpec {
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
        prompt: Some(format!("{TUTOR_PROMPT}\n\n## 本阶段任务\n{specific_prompt}")),
    }
}

fn gated(mut stage: StageSpec, gate: &str) -> StageSpec {
    stage.gate = Some(gate.into());
    stage
}

/// The conversational entry point: after the learner and the coordinator agree
/// on the practice setup, this tool creates the learning run and starts its
/// intake stage. The dashboard then picks up the new run automatically.
pub struct StartPracticeTool {
    pub state: Arc<ServerState>,
}

#[derive(Deserialize)]
struct StartPracticeArgs {
    topic: String,
    #[serde(default)]
    materials: Option<String>,
    #[serde(default)]
    syllabus: Option<String>,
    #[serde(default)]
    past_papers: Option<String>,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    goal: Option<String>,
    #[serde(default)]
    question_count: Option<u32>,
    #[serde(default)]
    time_limit_minutes: Option<u32>,
}

#[async_trait]
impl Tool for StartPracticeTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            "start_practice",
            "Start a timed practice session for the learning assistant. First collect the topic/exam, \
             knowledge points or materials, syllabus, past papers, question count, and time limit \
             from the user; use ask_user one question at a time and provide options where the answer \
             is a small set; summarize the setup for confirmation; then call this tool. It creates a \
             learning run and starts the intake stage, so the dashboard shows the materials, \
             knowledge points, timed practice, analysis, and report.",
        )
        .with_param(
            "topic",
            "string",
            "The topic or exam the learner wants to practice. Required.",
        )
        .with_param(
            "materials",
            "string",
            "Knowledge points or study material: a project-relative path, URL, or pasted text.",
        )
        .with_param(
            "syllabus",
            "string",
            "Exam syllabus: a project-relative path, URL, or pasted text.",
        )
        .with_param(
            "past_papers",
            "string",
            "Past exam papers: a project-relative path, URL, or pasted text.",
        )
        .with_param(
            "level",
            "string",
            "Learner level: beginner, intermediate, or advanced.",
        )
        .with_param("goal", "string", "Score or skill target, when the learner states one.")
        .with_param(
            "question_count",
            "integer",
            "Number of questions in the timed set (default 20).",
        )
        .with_param(
            "time_limit_minutes",
            "integer",
            "Time limit in minutes (default 30).",
        )
        .with_required("topic")
    }

    async fn run(&self, args: &Value, ctx: &ToolCtx) -> Result<String, ToolError> {
        let args: StartPracticeArgs = serde_json::from_value(args.clone()).map_err(|error| {
            ToolError::InvalidArgs(format!(
                "start_practice expects a topic plus optional materials/syllabus/past_papers: {error}"
            ))
        })?;
        let topic = args.topic.trim();
        if topic.is_empty() {
            return Err(ToolError::InvalidArgs(
                "start_practice requires a non-empty `topic`".into(),
            ));
        }
        let question_count = args.question_count.unwrap_or(20).clamp(1, 200);
        let time_limit_minutes = args.time_limit_minutes.unwrap_or(30).clamp(1, 600);
        let input = json!({
            "topic": topic,
            "materials": args.materials.unwrap_or_default().trim(),
            "syllabus": args.syllabus.unwrap_or_default().trim(),
            "past_papers": args.past_papers.unwrap_or_default().trim(),
            "level": args.level.unwrap_or_default().trim(),
            "goal": args.goal.unwrap_or_default().trim(),
            "question_count": question_count,
            "time_limit_minutes": time_limit_minutes,
        });
        let created = crate::runs::create_run(
            &self.state,
            ctx.project_id.as_str(),
            "assistant",
            &CreatePluginRunRequest {
                entry_id: Some("learning".into()),
                input,
            },
        )
        .await
        .map_err(|error| ToolError::Execution(error.to_string()))?;
        let run_id = created.id;
        if let Err(error) = crate::runs::start_run(
            &self.state,
            ctx.project_id.as_str(),
            "assistant",
            &run_id,
            &StartPluginRunRequest {
                model: None,
                stage: Some("intake".into()),
            },
        )
        .await
        {
            return Err(ToolError::Execution(format!(
                "created learning run {run_id} but could not start it: {error}"
            )));
        }
        Ok(format!(
            "Started timed practice run {run_id} (topic: {topic}, {question_count} questions, \
             {time_limit_minutes} minutes). The learning dashboard will show the materials stage; \
             tell the learner to open the Assistant dashboard and continue through the knowledge \
             points and timed practice stages. The practice timer starts when they click \
             \"Start timed practice\"."
        ))
    }
}

pub fn extension() -> AppExtension {
    AppExtension {
        routes,
        tools,
    }
}

async fn list_plugins(State(state): State<Arc<ServerState>>) -> Json<ApiResult<PluginsResponse>> {
    Json(ApiResult::ok(crate::catalog::list(&state).await))
}

fn routes(state: Arc<ServerState>) -> Router {
    Router::new()
        .route("/plugins", get(list_plugins))
        .route("/plugins/{name}/ui/{*path}", get(crate::ui::ui_asset))
        .route(
            "/projects/{pid}/plugins/{plugin}/runs",
            get(crate::runs_api::list).post(crate::runs_api::create),
        )
        .route(
            "/projects/{pid}/plugins/{plugin}/runs/{run}",
            get(crate::runs_api::get),
        )
        .route(
            "/projects/{pid}/plugins/{plugin}/runs/{run}/start",
            post(crate::runs_api::start),
        )
        .route(
            "/projects/{pid}/plugins/{plugin}/runs/{run}/approve",
            post(crate::runs_api::approve),
        )
        .route(
            "/projects/{pid}/plugins/{plugin}/runs/{run}/reject",
            post(crate::runs_api::reject),
        )
        .route(
            "/projects/{pid}/plugins/{plugin}/runs/{run}/artifacts/{artifact}",
            get(crate::runs_api::read_artifact).put(crate::runs_api::write_artifact),
        )
        .route(
            "/projects/{pid}/plugins/{plugin}/runs/{run}/directories",
            post(crate::runs_api::create_directory),
        )
        .route(
            "/projects/{pid}/plugins/{plugin}/runs/{run}/uploads",
            post(crate::runs_api::upload),
        )
        .route(
            "/projects/{pid}/plugins/{plugin}/runs/{run}/uploads/{*rel}",
            get(crate::runs_api::serve_upload).delete(crate::runs_api::delete_upload),
        )
        .with_state(state)
}

fn tools(state: Arc<ServerState>, _ctx: &AgentContext) -> Vec<AgentTool> {
    vec![AgentTool {
        agent: "main".into(),
        tool: Arc::new(StartPracticeTool { state }),
    }]
}

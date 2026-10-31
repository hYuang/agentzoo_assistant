//! Paper-reading vertical plugin.

use std::sync::OnceLock;

use serde_json::json;
use zoo_plugins::{
    ArtifactSpec, ChatTaskSpec, GateSpec, Plugin, PluginEntry, PluginManifest, PluginUiPage,
    RoleKind, RoleSpec, StageSpec, WorkflowSpec, PLUGIN_API_VERSION,
};

pub struct PaperPlugin;

impl Plugin for PaperPlugin {
    fn manifest(&self) -> &PluginManifest {
        static M: OnceLock<PluginManifest> = OnceLock::new();
        M.get_or_init(|| PluginManifest {
            id: "paper".into(),
            name: "Paper".into(),
            version: "0.1.0".into(),
            api_version: PLUGIN_API_VERSION,
            description: Some(
                "Paper reading: extract, summarize, compare, and review research papers.".into(),
            ),
            capabilities: vec!["workflow".into(), "artifacts".into()],
            requires: vec!["llm".into(), "file_read".into(), "pdf_extract".into()],
        })
    }

    fn artifact_types(&self) -> Vec<ArtifactSpec> {
        vec![
            artifact("paper", "Paper", "paper/"),
            artifact("extraction", "Extraction", "extraction.json"),
            artifact("analysis", "Analysis", "analysis.json"),
            artifact("comparison", "Comparison", "comparison.json"),
            artifact("review", "Review", "review.md"),
        ]
    }

    fn workflows(&self) -> Vec<WorkflowSpec> {
        vec![WorkflowSpec {
            id: "paper_flow".into(),
            name: "Paper reading".into(),
            roles: vec![
                role("researcher", "Researcher", RoleKind::Agent, vec![
                    "file_read",
                    "pdf_extract",
                ], vec![]),
                role("analyst", "Analyst", RoleKind::Agent, vec![
                    "long_form_writing",
                    "data_analysis",
                    "file_write",
                ], vec![]),
                role("reviewer", "Reviewer", RoleKind::Human, vec![], vec!["review"]),
            ],
            gates: vec![gate("review", "Review approval", "reviewer", Some("synthesize"))],
            stages: vec![
                stage("ingest", "Ingest", "researcher", vec!["file_read"], vec![], vec!["paper"]),
                stage("extract", "Extract", "researcher", vec!["pdf_extract"], vec!["ingest"], vec!["extraction"]),
                stage("analyze", "Analyze", "analyst", vec!["data_analysis"], vec!["extract"], vec!["analysis"]),
                stage("synthesize", "Synthesize", "analyst", vec!["long_form_writing"], vec!["analyze"], vec!["comparison", "review"]),
            ],
        }]
    }

    fn entries(&self) -> Vec<PluginEntry> {
        vec![PluginEntry {
            id: "paper".into(),
            title: "Paper Reading".into(),
            description: "Summarize a paper, compare related work, and list revision advice.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "files": { "type": "array", "items": { "type": "string" } },
                    "direction": { "type": "string" },
                    "focus": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["files"]
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "review": { "type": "string" },
                    "comparison": { "type": "string" }
                }
            }),
            ui_page: Some("index.html".into()),
        }]
    }

    fn ui_pages(&self) -> Vec<PluginUiPage> {
        vec![PluginUiPage {
            id: "paper".into(),
            title: "Paper".into(),
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

    fn chat_tasks(&self) -> Vec<ChatTaskSpec> {
        vec![
            ChatTaskSpec {
                id: "summarize_paper".into(),
                label_zh: "总结这篇论文".into(),
                label_en: "Summarize this paper".into(),
                prompt_zh: "请阅读项目中的论文，总结研究问题、方法、主要结论和局限，并指出最值得关注的图表。".into(),
                prompt_en: "Read the paper in this project. Summarize the research question, method, main findings, and limitations, and point out the most important figures.".into(),
            },
            ChatTaskSpec {
                id: "compare_papers".into(),
                label_zh: "对比多篇论文".into(),
                label_en: "Compare papers".into(),
                prompt_zh: "请对比项目中的多篇论文：问题设定、方法差异、实验设置和结论的异同，输出对比表格。".into(),
                prompt_en: "Compare the papers in this project: problem setup, method differences, experimental setup, and how their conclusions agree or diverge. Output a comparison table.".into(),
            },
            ChatTaskSpec {
                id: "review_paper".into(),
                label_zh: "写一份审稿意见".into(),
                label_en: "Write a review".into(),
                prompt_zh: "请以审稿人的角度审阅项目中的论文，给出贡献评价、方法质疑、实验缺陷和修改建议。".into(),
                prompt_en: "Review the paper in this project from a reviewer's perspective: assess contributions, challenge the method, flag experimental gaps, and suggest revisions.".into(),
            },
        ]
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

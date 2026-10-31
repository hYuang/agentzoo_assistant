//! Audiobook vertical plugin.
//!
//! The plugin owns audiobook artifacts and the production workflow. It does
//! not register agents: the host resolves roles against the existing agent
//! capability pool.
//!
//! The workflow is data-driven: `tasks.yaml` defines each task (role,
//! capabilities, input/output artifacts, prompt file) and `flow.yaml` defines
//! how tasks relate (roles, gates, ordering, fan-out). `workflows()` parses
//! both and joins them on task id.

use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::{json, Value};
use zoo_plugins::{
    ArtifactSpec, ChatTaskSpec, GateSpec, Plugin, PluginEntry, PluginManifest, PluginUiPage,
    RoleKind, RoleSpec, RunInit, StageSpec, WorkflowSpec, PLUGIN_API_VERSION,
};

pub struct AudiobookPlugin;

impl Plugin for AudiobookPlugin {
    fn manifest(&self) -> &PluginManifest {
        static M: OnceLock<PluginManifest> = OnceLock::new();
        M.get_or_init(|| PluginManifest {
            id: "audiobook".into(),
            name: "Audiobook".into(),
            version: "0.1.0".into(),
            api_version: PLUGIN_API_VERSION,
            description: Some(
                "Audiobook production: story bible, script adaptation, casting, synthesis, post, QC, delivery."
                    .into(),
            ),
            capabilities: vec!["workflow".into(), "artifacts".into(), "audio_production".into()],
            requires: vec![
                "llm".into(),
                "tts".into(),
                "audio_mix".into(),
                "file_write".into(),
            ],
        })
    }

    fn artifact_types(&self) -> Vec<ArtifactSpec> {
        vec![
            artifact(
                "directions",
                "Direction Candidates",
                "directions.json",
                include_str!("../schemas/directions.json"),
            ),
            artifact(
                "project",
                "Project",
                "project.json",
                include_str!("../schemas/project.json"),
            ),
            artifact(
                "story_bible",
                "Story Bible",
                "story_bible.json",
                include_str!("../schemas/story_bible.json"),
            ),
            artifact(
                "outline",
                "Outline",
                "outline.json",
                include_str!("../schemas/outline.json"),
            ),
            artifact(
                "script",
                "Script",
                "script.jsonl",
                include_str!("../schemas/script_line.json"),
            ),
            artifact(
                "asset_index",
                "Asset Index",
                "assets/index.json",
                include_str!("../schemas/asset_index.json"),
            ),
            artifact(
                "casting",
                "Casting",
                "casting.json",
                include_str!("../schemas/casting.json"),
            ),
            artifact("audio", "Audio", "audio/", ""),
            artifact(
                "qc",
                "QC Report",
                "qc.json",
                include_str!("../schemas/qc.json"),
            ),
            artifact("package", "Delivery Package", "package/", ""),
        ]
    }

    fn workflows(&self) -> Vec<WorkflowSpec> {
        vec![build_workflow()]
    }

    fn entries(&self) -> Vec<PluginEntry> {
        vec![PluginEntry {
            id: "audiobook".into(),
            title: "Audiobook".into(),
            description: "Turn a story into a mastered audiobook package.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": ["discover", "idea", "outline", "manuscript", "series", "blank"],
                        "default": "discover"
                    },
                    "title": { "type": "string" },
                    "constraints": { "type": "string", "description": "Optional genre, audience, length, tone, or references when starting from zero" },
                    "idea": { "type": "string", "description": "Premise, genre, audience, constraints" },
                    "outline": { "type": "string", "description": "Existing outline, character notes, or series bible" },
                    "source": { "type": "string", "description": "Project-relative path to an existing manuscript" },
                    "language": { "type": "string", "default": "zh" },
                    "style": { "type": "string", "description": "Narrative style and delivery constraints" },
                    "target_platform": { "type": "string", "description": "Delivery target and loudness spec" },
                    "format": { "type": "string", "enum": ["wav", "mp3"], "default": "wav" }
                }
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "audio": { "type": "string" },
                    "metadata": { "type": "string" },
                    "qc": { "type": "string" },
                    "package": { "type": "string" }
                }
            }),
            ui_page: Some("index.html".into()),
        }]
    }

    fn ui_pages(&self) -> Vec<PluginUiPage> {
        vec![
            PluginUiPage {
                id: "studio".into(),
                title: "Audiobook Studio".into(),
                page: "index.html".into(),
                slot: "plugin.dashboard".into(),
            },
            PluginUiPage {
                id: "script".into(),
                title: "Script".into(),
                page: "script.html".into(),
                slot: "plugin.artifact".into(),
            },
            PluginUiPage {
                id: "casting".into(),
                title: "Casting".into(),
                page: "casting.html".into(),
                slot: "plugin.artifact".into(),
            },
        ]
    }

    fn init_run(&self, init: &RunInit) -> Result<(), zoo_plugins::PluginError> {
        self.seed_run(init)
    }

    fn chat_tasks(&self) -> Vec<ChatTaskSpec> {
        vec![
            ChatTaskSpec {
                id: "story_diagnosis".into(),
                label_zh: "故事诊断".into(),
                label_en: "Story diagnosis".into(),
                prompt_zh: "请阅读当前项目的故事圣经、人物表和分章大纲，检查逻辑矛盾、人物动机和节奏问题，输出具体修改建议。".into(),
                prompt_en: "Read the current story bible, character notes, and outline. Check for logical contradictions, weak motivations, and pacing problems, then give concrete fixes.".into(),
            },
            ChatTaskSpec {
                id: "continue_chapter".into(),
                label_zh: "续写下一章".into(),
                label_en: "Continue next chapter".into(),
                prompt_zh: "请基于当前分章大纲续写下一章，保持人物口吻和既有设定；把结果写入项目工件，或给出可直接采用的文本。".into(),
                prompt_en: "Continue the next chapter from the current outline. Keep the established voices and canon, then write the result into the project artifacts or return usable text.".into(),
            },
            ChatTaskSpec {
                id: "polish_narration".into(),
                label_zh: "润色旁白".into(),
                label_en: "Polish narration".into(),
                prompt_zh: "请挑选当前章节中最适合朗读的旁白段落，做口语化润色，保留原意，并标注适合的语速和停顿。".into(),
                prompt_en: "Pick the narration passages in the current chapter that work best aloud. Polish them for spoken delivery, preserve the meaning, and note pacing and pauses.".into(),
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

fn artifact(id: &str, title: &str, path: &str, schema: &str) -> ArtifactSpec {
    ArtifactSpec {
        id: id.into(),
        title: title.into(),
        path: Some(path.into()),
        schema: if schema.trim().is_empty() {
            None
        } else {
            serde_json::from_str(schema).ok()
        },
        version: 1,
    }
}

/// Task definitions from `tasks.yaml`: what each task is and what it reads
/// and writes.
#[derive(Debug, Deserialize)]
struct TaskFile {
    tasks: Vec<TaskDef>,
}

#[derive(Debug, Deserialize)]
struct TaskDef {
    id: String,
    name: String,
    role: String,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    outputs: Vec<String>,
    #[serde(default)]
    prompt_file: Option<String>,
}

/// Relationship definitions from `flow.yaml`: roles, gates, and how tasks
/// are ordered.
#[derive(Debug, Deserialize)]
struct FlowFile {
    workflow: WorkflowMeta,
    #[serde(default)]
    roles: Vec<RoleDef>,
    #[serde(default)]
    gates: Vec<GateDef>,
    flow: Vec<FlowEdge>,
}

#[derive(Debug, Deserialize)]
struct WorkflowMeta {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct RoleDef {
    id: String,
    name: String,
    #[serde(default = "default_role_kind")]
    kind: RoleKind,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    can_approve: Vec<String>,
}

fn default_role_kind() -> RoleKind {
    RoleKind::Agent
}

#[derive(Debug, Deserialize)]
struct GateDef {
    id: String,
    name: String,
    approver: String,
    #[serde(default)]
    on_reject: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FlowEdge {
    id: String,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    gate: Option<String>,
    #[serde(default)]
    foreach: Option<String>,
}

/// Parse both YAML files and join them on task id into a `WorkflowSpec`.
/// Panics when the definitions are inconsistent; the result is cached.
fn build_workflow() -> WorkflowSpec {
    WORKFLOW.get_or_init(|| {
        let tasks: TaskFile = parse_yaml(include_str!("../tasks.yaml"), "tasks.yaml");
        let flow: FlowFile = parse_yaml(include_str!("../flow.yaml"), "flow.yaml");
        let mut errors = Vec::new();
        let workflow = join(tasks, flow, &mut errors);
        if !errors.is_empty() {
            panic!(
                "invalid audiobook workflow definition:\n{}",
                errors.join("\n")
            );
        }
        workflow
    })
    .clone()
}

fn parse_yaml<T: for<'de> Deserialize<'de>>(text: &str, name: &str) -> T {
    serde_yaml::from_str(text)
        .unwrap_or_else(|error| panic!("invalid {name}: {error}"))
}

static WORKFLOW: OnceLock<WorkflowSpec> = OnceLock::new();

/// Merge task definitions with flow edges and validate every reference.
fn join(tasks: TaskFile, flow: FlowFile, errors: &mut Vec<String>) -> WorkflowSpec {
    let task_ids: Vec<&str> = tasks.tasks.iter().map(|task| task.id.as_str()).collect();
    let role_ids: Vec<&str> = flow.roles.iter().map(|role| role.id.as_str()).collect();
    let gate_ids: Vec<&str> = flow.gates.iter().map(|gate| gate.id.as_str()).collect();
    let artifacts = AudiobookPlugin.artifact_types();
    let artifact_ids: Vec<&str> = artifacts
        .iter()
        .map(|artifact| artifact.id.as_str())
        .collect();

    let mut seen = std::collections::HashSet::new();
    let stages: Vec<StageSpec> = flow
        .flow
        .into_iter()
        .map(|edge| {
            if !seen.insert(edge.id.clone()) {
                errors.push(format!("duplicate task id {}", edge.id));
            }
            let task = tasks
                .tasks
                .iter()
                .find(|task| task.id == edge.id)
                .unwrap_or_else(|| {
                    errors.push(format!("flow references unknown task {}", edge.id));
                    &tasks.tasks[0]
                });
            if !role_ids.contains(&task.role.as_str()) {
                errors.push(format!(
                    "task {} references unknown role {}",
                    task.id, task.role
                ));
            }
            for dependency in &edge.depends_on {
                if !task_ids.contains(&dependency.as_str()) {
                    errors.push(format!(
                        "task {} depends on unknown task {dependency}",
                        task.id
                    ));
                }
            }
            if let Some(gate) = &edge.gate {
                if !gate_ids.contains(&gate.as_str()) {
                    errors.push(format!("task {} references unknown gate {gate}", task.id));
                }
            }
            for artifact in task.inputs.iter().chain(task.outputs.iter()) {
                if !artifact_ids.contains(&artifact.as_str()) {
                    errors.push(format!(
                        "task {} references unknown artifact {artifact}",
                        task.id
                    ));
                }
            }
            let prompt = task
                .prompt_file
                .as_deref()
                .map(prompt_text)
                .unwrap_or_default();
            StageSpec {
                id: task.id.clone(),
                name: task.name.clone(),
                role: task.role.clone(),
                requires: task.requires.clone(),
                depends_on: edge.depends_on,
                inputs: task.inputs.clone(),
                outputs: task.outputs.clone(),
                gate: edge.gate,
                foreach: edge.foreach,
                prompt: Some(prompt),
            }
        })
        .collect();

    for gate in &flow.gates {
        if !role_ids.contains(&gate.approver.as_str()) {
            errors.push(format!(
                "gate {} references unknown role {}",
                gate.id, gate.approver
            ));
        }
        if let Some(target) = &gate.on_reject {
            if !task_ids.contains(&target.as_str()) {
                errors.push(format!("gate {} rejects to unknown task {target}", gate.id));
            }
        }
    }
    for id in &task_ids {
        if !stages.iter().any(|stage| &stage.id == *id) {
            errors.push(format!("task {id} has no flow entry"));
        }
    }
    if has_cycle(&stages) {
        errors.push("stage graph has a cycle".into());
    }

    WorkflowSpec {
        id: flow.workflow.id,
        name: flow.workflow.name,
        roles: flow
            .roles
            .into_iter()
            .map(|role| RoleSpec {
                id: role.id,
                name: role.name,
                kind: role.kind,
                capabilities: role.capabilities,
                can_approve: role.can_approve,
            })
            .collect(),
        gates: flow
            .gates
            .into_iter()
            .map(|gate| GateSpec {
                id: gate.id,
                name: gate.name,
                approver: gate.approver,
                on_reject: gate.on_reject,
            })
            .collect(),
        stages,
    }
}

/// Stage prompts are inlined at compile time so the server never depends on
/// its working directory to find them.
fn prompt_text(path: &str) -> String {
    match path {
        "prompts/stages/discover.md" => include_str!("../prompts/stages/discover.md"),
        "prompts/stages/brief.md" => include_str!("../prompts/stages/brief.md"),
        "prompts/stages/source.md" => include_str!("../prompts/stages/source.md"),
        "prompts/stages/story.md" => include_str!("../prompts/stages/story.md"),
        "prompts/stages/script.md" => include_str!("../prompts/stages/script.md"),
        "prompts/stages/casting.md" => include_str!("../prompts/stages/casting.md"),
        "prompts/stages/synthesis.md" => include_str!("../prompts/stages/synthesis.md"),
        "prompts/stages/post.md" => include_str!("../prompts/stages/post.md"),
        "prompts/stages/qc.md" => include_str!("../prompts/stages/qc.md"),
        "prompts/stages/delivery.md" => include_str!("../prompts/stages/delivery.md"),
        other => panic!("unknown prompt file {other}"),
    }
    .to_string()
}

/// Iterative DFS over the dependency edges; a dependency already on the
/// current path is a cycle.
fn has_cycle(stages: &[StageSpec]) -> bool {
    let mut done: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for stage in stages {
        if done.contains(stage.id.as_str()) {
            continue;
        }
        let mut path: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut stack: Vec<(&str, usize)> = vec![(stage.id.as_str(), 0)];
        while let Some((current, next)) = stack.pop() {
            if done.contains(current) {
                continue;
            }
            if path.contains(current) {
                return true;
            }
            let dependencies: Vec<&str> = stages
                .iter()
                .find(|s| s.id == current)
                .map(|s| s.depends_on.iter().map(|d| d.as_str()).collect())
                .unwrap_or_default();
            if next < dependencies.len() {
                stack.push((current, next + 1));
                let dependency = dependencies[next];
                if !done.contains(dependency) {
                    path.insert(current);
                    stack.push((dependency, 0));
                }
            } else {
                path.remove(current);
                done.insert(current);
            }
        }
    }
    false
}

impl AudiobookPlugin {
    /// Seed the input contract and the empty asset catalog for a new run.
    fn seed_run(&self, init: &RunInit) -> Result<(), zoo_plugins::PluginError> {
        let text = |key: &str| {
            init.input
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string()
        };
        let mode = init
            .input
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("discover")
            .trim()
            .to_string();
        let source = text("source");
        let idea = text("idea");
        let outline = text("outline");
        let title = text("title");
        let constraints = text("constraints");
        let project = json!({
            "mode": mode,
            "title": title,
            "source": source,
            "idea": idea,
            "outline": outline,
            "constraints": constraints,
            "language": init.input.get("language").cloned().unwrap_or(json!("zh")),
            "style": init.input.get("style").cloned().unwrap_or(json!("")),
            "target_platform": init.input.get("target_platform").cloned().unwrap_or(json!("")),
            "format": init.input.get("format").cloned().unwrap_or(json!("wav")),
            "created_at": chrono::Utc::now().to_rfc3339(),
        });
        write_json(&init.artifact_dir.join("project.json"), &project)?;
        write_json(
            &init.artifact_dir.join("assets").join("index.json"),
            &json!({ "assets": [] }),
        )?;
        Ok(())
    }
}

fn write_json(path: &std::path::Path, value: &Value) -> Result<(), zoo_plugins::PluginError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| zoo_plugins::PluginError::lifecycle("audiobook", error.to_string()))?;
    std::fs::write(path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "zoo-audiobook-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn init_for(dir: &PathBuf, input: Value) -> RunInit {
        RunInit {
            plugin_id: "audiobook".into(),
            project_id: "p-1".into(),
            run_id: "r-1".into(),
            entry_id: Some("audiobook".into()),
            project_root: dir.clone(),
            run_dir: dir.clone(),
            artifact_dir: dir.clone(),
            input,
        }
    }

    #[test]
    fn seeds_project_and_asset_index() {
        let dir = temp_dir("seed");
        std::fs::create_dir_all(&dir).unwrap();
        let init = init_for(
            &dir,
            json!({ "source": "story.md", "language": "zh", "format": "mp3" }),
        );
        AudiobookPlugin.init_run(&init).unwrap();
        let project: Value =
            serde_json::from_slice(&std::fs::read(dir.join("project.json")).unwrap()).unwrap();
        assert_eq!(project["source"], "story.md");
        assert_eq!(project["format"], "mp3");
        let assets: Value = serde_json::from_slice(
            &std::fs::read(dir.join("assets").join("index.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(assets["assets"], json!([]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn allows_an_empty_discovery_run() {
        let dir = temp_dir("empty-discovery");
        std::fs::create_dir_all(&dir).unwrap();
        let init = init_for(&dir, json!({}));
        AudiobookPlugin.init_run(&init).unwrap();
        let project: Value =
            serde_json::from_slice(&std::fs::read(dir.join("project.json")).unwrap()).unwrap();
        assert_eq!(project["mode"], "discover");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn seeds_from_an_idea_without_a_manuscript() {
        let dir = temp_dir("idea");
        std::fs::create_dir_all(&dir).unwrap();
        let init = init_for(
            &dir,
            json!({ "mode": "idea", "title": "雾港", "idea": "一个关于失踪录音带的故事" }),
        );
        AudiobookPlugin.init_run(&init).unwrap();
        let project: Value =
            serde_json::from_slice(&std::fs::read(dir.join("project.json")).unwrap()).unwrap();
        assert_eq!(project["source"], "");
        assert_eq!(project["idea"], "一个关于失踪录音带的故事");
        assert_eq!(project["title"], "雾港");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn workflow_loads_from_yaml() {
        let workflow = build_workflow();
        assert_eq!(workflow.id, "audiobook_production");
        assert_eq!(workflow.stages.len(), 10);
        assert_eq!(workflow.roles.len(), 7);
        assert_eq!(workflow.gates.len(), 6);
    }

    #[test]
    fn workflow_stages_carry_inputs_and_outputs() {
        let workflow = build_workflow();
        let script = workflow
            .stages
            .iter()
            .find(|stage| stage.id == "script")
            .unwrap();
        assert_eq!(script.inputs, vec!["story_bible", "outline", "asset_index"]);
        assert_eq!(script.outputs, vec!["script"]);
        assert!(script.prompt.as_deref().unwrap().contains("剧本改编"));
    }

    #[test]
    fn workflow_flow_matches_the_production_chain() {
        let workflow = build_workflow();
        let stage = |id: &str| {
            workflow
                .stages
                .iter()
                .find(|stage| stage.id == id)
                .unwrap()
        };
        assert!(stage("discover").depends_on.is_empty());
        assert_eq!(stage("brief").depends_on, vec!["discover"]);
        assert_eq!(stage("story").depends_on, vec!["source"]);
        assert_eq!(stage("synthesis").depends_on, vec!["casting"]);
        assert_eq!(stage("delivery").depends_on, vec!["qc"]);
        assert_eq!(stage("synthesis").foreach.as_deref(), Some("script.chapters"));
        assert_eq!(stage("qc").gate.as_deref(), Some("final"));
    }

    #[test]
    fn workflow_has_no_cycle() {
        let workflow = build_workflow();
        assert!(!has_cycle(&workflow.stages));
    }

    #[test]
    fn detects_a_cycle() {
        let stages = vec![
            StageSpec {
                id: "a".into(),
                name: "a".into(),
                role: "r".into(),
                requires: vec![],
                depends_on: vec!["b".into()],
                inputs: vec![],
                outputs: vec![],
                gate: None,
                foreach: None,
                prompt: None,
            },
            StageSpec {
                id: "b".into(),
                name: "b".into(),
                role: "r".into(),
                requires: vec![],
                depends_on: vec!["a".into()],
                inputs: vec![],
                outputs: vec![],
                gate: None,
                foreach: None,
                prompt: None,
            },
        ];
        assert!(has_cycle(&stages));
    }
}

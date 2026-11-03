//! Workflow run persistence.
//!
//! A run is one execution of the app workflow. Runtime state stays under
//! `.zoo_agent` while canonical artifacts live in the project so they can be
//! committed to Git:
//!
//! ```text
//! <project>/.zoo_agent/assistant/runs/<run>/run.json   runtime state
//! <project>/.zoo_agent/assistant/uploads/              private material
//! <project>/assistant/<artifact path>                  tracked artifacts
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::api::{
    ArtifactContent, CreateDirectory, CreateDirectoryResponse, CreateRun, RejectRun, Run,
    RunArtifact as RunArtifactInfo, Runs, StartRun, UploadFile, UploadFileResponse, WriteArtifact,
};
use crate::workflow::{
    ArtifactStatus, RoleKind, RunArtifact, RunStatus, StageSpec, WorkflowApp, WorkflowRun,
    WorkflowSpec,
};
use axum::http::HeaderMap;
use axum::response::Response;
use base64::Engine;
use uuid::Uuid;
use zoo_shared::api::{CreateSessionRequest, ModelSelection, SendMessageRequest};
use zoo_shared::message::ProjectId;

use zoo_server::service::packs::safe_join;
use zoo_server::service::{files, sessions, ServerState};
use zoo_server::ApiError;

/// The runtime state directory for one run.
pub fn run_dir(project_root: &Path, run_id: &str) -> PathBuf {
    project_root
        .join(".zoo_agent")
        .join("assistant")
        .join("runs")
        .join(run_id)
}

/// The canonical, Git-trackable artifact directory: `<project>/assistant/`.
pub fn artifact_dir(project_root: &Path) -> PathBuf {
    project_root.join("assistant")
}

/// The app's upload root: `<project>/.zoo_agent/assistant/uploads/`.
pub fn uploads_dir(project_root: &Path) -> PathBuf {
    project_root
        .join(".zoo_agent")
        .join("assistant")
        .join("uploads")
}

/// Create a run. The run starts empty; stages write its artifacts through
/// [`write_artifact`].
pub async fn create_run(state: &ServerState, pid: &str, body: &CreateRun) -> Result<Run, ApiError> {
    let project = state
        .project(&ProjectId::from(pid))
        .await
        .ok_or(ApiError::ProjectNotFound)?;
    let app = crate::AssistantApp;
    let artifacts = app.artifact_types();
    let mut run = WorkflowRun::new(
        Uuid::new_v4().to_string(),
        pid,
        body.input.clone(),
        &artifacts,
    );
    run.workflow_id = app.workflows().first().map(|workflow| workflow.id.clone());
    if run.session_id.is_none() {
        let created = sessions::create_session(
            state,
            pid,
            &CreateSessionRequest {
                agent: "main".into(),
            },
        )
        .await?;
        run.session_id = Some(created.session_id);
    }
    let dir = run_dir(&project.path, &run.id);
    let artifact_root = artifact_dir(&project.path);
    tokio::fs::create_dir_all(&dir).await.map_err(internal)?;
    tokio::fs::create_dir_all(&artifact_root)
        .await
        .map_err(internal)?;
    // A new run starts from a clean slate: drop any artifacts left behind by
    // a previous run so they are not mistaken for this run's output.
    for spec in &artifacts {
        if let Some(rel) = spec.path.as_deref().filter(|p| !p.is_empty()) {
            let path = artifact_root.join(rel);
            if path.is_file() {
                let _ = tokio::fs::remove_file(&path).await;
            }
        }
    }
    save_run(&dir, &run).await?;
    refresh_artifacts(&mut run, &artifact_root).await;
    save_run(&dir, &run).await?;
    Ok(to_run(run))
}

/// List the app's runs in one project, most recently updated first.
pub async fn list_runs(state: &ServerState, pid: &str) -> Result<Runs, ApiError> {
    let project = state
        .project(&ProjectId::from(pid))
        .await
        .ok_or(ApiError::ProjectNotFound)?;
    let runs_dir = project
        .path
        .join(".zoo_agent")
        .join("assistant")
        .join("runs");
    let mut runs = Vec::new();
    let mut entries = match tokio::fs::read_dir(&runs_dir).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Runs { runs });
        }
        Err(error) => return Err(internal(error)),
    };
    while let Some(entry) = entries.next_entry().await.map_err(internal)? {
        if !entry.file_type().await.map_err(internal)?.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        if let Ok(run) = load_run(&entry.path(), &id).await {
            runs.push(to_run(run));
        }
    }
    runs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(Runs { runs })
}

/// Load one run.
pub async fn get_run(state: &ServerState, pid: &str, run_id: &str) -> Result<Run, ApiError> {
    let (_, _, run) = load_for_project(state, pid, run_id).await?;
    Ok(to_run(run))
}

/// Read one artifact. Text is returned as UTF-8; binary is base64-encoded.
pub async fn read_artifact(
    state: &ServerState,
    pid: &str,
    run_id: &str,
    artifact_id: &str,
) -> Result<ArtifactContent, ApiError> {
    let (project, _, run) = load_for_project(state, pid, run_id).await?;
    let artifact_root = artifact_dir(&project.path);
    let artifact = run
        .artifact(artifact_id)
        .ok_or_else(|| ApiError::FileNotFound(artifact_id.to_string()))?;
    if artifact.is_dir() {
        return Err(ApiError::InvalidRequest(format!(
            "artifact {artifact_id} is a directory"
        )));
    }
    let path = safe_join(&artifact_root, &artifact.path)
        .ok_or_else(|| ApiError::InvalidRequest("unsafe artifact path".into()))?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| ApiError::FileNotFound(artifact.path.clone()))?;
    match String::from_utf8(bytes.clone()) {
        Ok(content) => Ok(ArtifactContent {
            content,
            encoding: "utf-8".into(),
        }),
        Err(_) => Ok(ArtifactContent {
            content: base64::engine::general_purpose::STANDARD.encode(bytes),
            encoding: "base64".into(),
        }),
    }
}

/// Write one artifact and return the updated run.
pub async fn write_artifact(
    state: &ServerState,
    pid: &str,
    run_id: &str,
    artifact_id: &str,
    body: &WriteArtifact,
) -> Result<Run, ApiError> {
    let (project, dir, mut run) = load_for_project(state, pid, run_id).await?;
    let artifact_root = artifact_dir(&project.path);
    let artifact = run
        .artifact(artifact_id)
        .cloned()
        .ok_or_else(|| ApiError::FileNotFound(artifact_id.to_string()))?;
    if artifact.is_dir() {
        return Err(ApiError::InvalidRequest(format!(
            "artifact {artifact_id} is a directory"
        )));
    }
    let path = safe_join(&artifact_root, &artifact.path)
        .ok_or_else(|| ApiError::InvalidRequest("unsafe artifact path".into()))?;
    let bytes = match body.encoding.as_deref() {
        None | Some("utf-8") | Some("utf8") => body.content.as_bytes().to_vec(),
        Some("base64") => base64::engine::general_purpose::STANDARD
            .decode(body.content.as_bytes())
            .map_err(|error| ApiError::InvalidRequest(format!("invalid base64: {error}")))?,
        Some(other) => {
            return Err(ApiError::InvalidRequest(format!(
                "unsupported encoding: {other}"
            )))
        }
    };
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(internal)?;
    }
    tokio::fs::write(&path, &bytes).await.map_err(internal)?;
    run.touch_artifact(artifact_id, bytes.len() as u64);
    save_run(&dir, &run).await?;
    Ok(to_run(run))
}

/// Upload a user-provided source file into the app's upload root
/// at `<project>/.zoo_agent/assistant/uploads/`.
pub async fn upload_file(
    state: &ServerState,
    pid: &str,
    run_id: &str,
    body: &UploadFile,
) -> Result<UploadFileResponse, ApiError> {
    if body.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest(
            "file name must not be empty".into(),
        ));
    }
    let (project, _, _) = load_for_project(state, pid, run_id).await?;
    let raw = body
        .data
        .split_once("base64,")
        .map(|(_, data)| data)
        .unwrap_or(&body.data);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .map_err(|error| ApiError::InvalidRequest(format!("invalid base64: {error}")))?;
    if bytes.is_empty() {
        return Err(ApiError::InvalidRequest("empty file".into()));
    }
    let name = files::sanitize_name(&body.name);
    let stored = format!("{}-{}", Uuid::new_v4(), name);
    let relative_dir = body
        .dir
        .as_deref()
        .map(sanitize_relative_dir)
        .transpose()?
        .unwrap_or_default();
    let dir = uploads_dir(&project.path).join(&relative_dir);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|error| ApiError::Internal(format!("failed to create upload dir: {error}")))?;
    tokio::fs::write(dir.join(&stored), &bytes)
        .await
        .map_err(|error| ApiError::Internal(format!("failed to write upload: {error}")))?;
    let relative_dir = relative_dir.to_string_lossy().replace('\\', "/");
    let path = if relative_dir.is_empty() {
        format!("assets/uploads/{stored}")
    } else {
        format!("assets/uploads/{relative_dir}/{stored}")
    };
    Ok(UploadFileResponse {
        path,
        name,
        size: bytes.len() as u64,
    })
}

/// Create one directory under the app's upload root. Empty and parent (`..`)
/// segments are rejected so the path cannot escape `assets/uploads/`.
pub async fn create_directory(
    state: &ServerState,
    pid: &str,
    run_id: &str,
    body: &CreateDirectory,
) -> Result<CreateDirectoryResponse, ApiError> {
    if body.path.trim().is_empty() {
        return Err(ApiError::InvalidRequest("empty directory path".into()));
    }
    let (project, _, _) = load_for_project(state, pid, run_id).await?;
    let relative = sanitize_relative_dir(&body.path)?;
    if relative.as_os_str().is_empty() {
        return Err(ApiError::InvalidRequest("empty directory path".into()));
    }
    let dir = uploads_dir(&project.path).join(&relative);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|error| ApiError::Internal(format!("failed to create directory: {error}")))?;
    Ok(CreateDirectoryResponse {
        path: format!(
            "assets/uploads/{}/",
            relative.to_string_lossy().replace('\\', "/")
        ),
    })
}

fn sanitize_relative_dir(value: &str) -> Result<PathBuf, ApiError> {
    let mut path = PathBuf::new();
    for segment in value.split(['/', '\\']) {
        let segment = segment.trim();
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return Err(ApiError::InvalidRequest("invalid directory path".into()));
        }
        path.push(files::sanitize_name(segment));
    }
    Ok(path)
}

/// Delete one uploaded file or directory under the run's upload root.
pub async fn delete_upload(
    state: &ServerState,
    pid: &str,
    run_id: &str,
    rel: &str,
) -> Result<(), ApiError> {
    if rel.trim().is_empty() {
        return Err(ApiError::InvalidRequest("empty upload path".into()));
    }
    let (project, _, _) = load_for_project(state, pid, run_id).await?;
    let base = uploads_dir(&project.path);
    let path = files::resolve_rel(&base, rel)?;
    delete_upload_path(&path).await?;
    Ok(())
}

async fn delete_upload_path(path: &Path) -> Result<(), ApiError> {
    let meta = tokio::fs::metadata(path)
        .await
        .map_err(|_| ApiError::FileNotFound(path.to_string_lossy().into()))?;
    if meta.is_dir() {
        tokio::fs::remove_dir_all(path).await.map_err(internal)?;
    } else {
        tokio::fs::remove_file(path).await.map_err(internal)?;
    }
    Ok(())
}

/// Serve a user-provided source file uploaded through [`upload_file`].
pub async fn serve_upload(
    state: &ServerState,
    pid: &str,
    run_id: &str,
    rel: &str,
    headers: &HeaderMap,
) -> Result<Response, ApiError> {
    let (project, _, _) = load_for_project(state, pid, run_id).await?;
    let base = uploads_dir(&project.path);
    let file = files::resolve_rel(&base, rel)?;
    let ext = file
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();
    files::serve_path(&file, rel, &ext, headers).await
}

/// Start the next ready stage. The run executes exactly one stage and stops
/// for review; it never chains the whole workflow.
pub async fn start_run(
    state: &Arc<ServerState>,
    pid: &str,
    run_id: &str,
    body: &StartRun,
) -> Result<Run, ApiError> {
    let (project, dir, mut run) = load_for_project(state, pid, run_id).await?;
    let artifacts = artifact_dir(&project.path);
    if matches!(run.status, RunStatus::Running | RunStatus::WaitingGate) {
        return Err(ApiError::InvalidRequest(format!(
            "run {} cannot start from {}",
            run.id,
            run.status.as_str()
        )));
    }
    if run.status == RunStatus::Completed && body.stage.is_none() {
        return Err(ApiError::InvalidRequest(
            "run is completed; name a stage to re-run it".into(),
        ));
    }
    let workflow = select_workflow(&run)?;
    if let Some(stage_id) = body.stage.as_deref() {
        if !workflow.stages.iter().any(|stage| stage.id == stage_id) {
            return Err(ApiError::InvalidRequest(format!(
                "unknown stage: {stage_id}"
            )));
        }
    }
    run.workflow_id = Some(workflow.id.clone());
    if run.session_id.is_none() {
        let created = sessions::create_session(
            state,
            pid,
            &CreateSessionRequest {
                agent: "main".into(),
            },
        )
        .await?;
        run.session_id = Some(created.session_id);
    }
    run.status = RunStatus::Running;
    run.stage = None;
    run.error = None;
    run.review_notes = None;
    run.updated_at = now();
    save_run(&dir, &run).await?;
    spawn_runner(
        state,
        dir,
        artifacts,
        run.clone(),
        workflow,
        body.model.clone(),
        body.stage.clone(),
    );
    Ok(to_run(run))
}

/// Approve the stage waiting at a human gate. It becomes completed, but the
/// next stage is not started until the user asks for it.
pub async fn approve_run(
    state: &Arc<ServerState>,
    pid: &str,
    run_id: &str,
    _body: &StartRun,
) -> Result<Run, ApiError> {
    let (_, dir, mut run) = load_for_project(state, pid, run_id).await?;
    if run.status != RunStatus::WaitingGate {
        return Err(ApiError::InvalidRequest(format!(
            "run {} is not waiting for approval",
            run.status.as_str()
        )));
    }
    let stage_id = run
        .stage
        .clone()
        .ok_or_else(|| ApiError::InvalidRequest("run has no stage to approve".into()))?;
    if !run.completed_stages.iter().any(|id| id == &stage_id) {
        run.completed_stages.push(stage_id);
    }
    run.status = RunStatus::Created;
    run.stage = None;
    run.error = None;
    run.review_notes = None;
    run.updated_at = now();
    save_run(&dir, &run).await?;
    Ok(to_run(run))
}

/// Reject the current stage review. The stage stays incomplete so the next
/// generate re-runs it with the feedback.
pub async fn reject_run(
    state: &ServerState,
    pid: &str,
    run_id: &str,
    body: &RejectRun,
) -> Result<Run, ApiError> {
    let (_, dir, mut run) = load_for_project(state, pid, run_id).await?;
    if run.status != RunStatus::WaitingGate {
        return Err(ApiError::InvalidRequest(format!(
            "run {} is not waiting for review",
            run.status.as_str()
        )));
    }
    run.status = RunStatus::Created;
    run.stage = None;
    run.error = None;
    run.review_notes = Some(body.reason.trim().to_string());
    run.updated_at = now();
    save_run(&dir, &run).await?;
    Ok(to_run(run))
}

fn spawn_runner(
    state: &Arc<ServerState>,
    dir: PathBuf,
    artifacts: PathBuf,
    initial: WorkflowRun,
    workflow: WorkflowSpec,
    model: Option<String>,
    stage: Option<String>,
) {
    let state = Arc::clone(state);
    tokio::spawn(async move {
        if let Err(error) = run_one_stage(
            &state,
            &dir,
            &artifacts,
            &initial,
            &workflow,
            model.as_deref(),
            stage.as_deref(),
        )
        .await
        {
            tracing::warn!(
                run = %initial.id,
                error = %error,
                "workflow stage failed"
            );
            mark_failed(&dir, &initial.id, &error).await;
        }
    });
}

async fn run_one_stage(
    state: &Arc<ServerState>,
    dir: &Path,
    artifacts: &Path,
    initial: &WorkflowRun,
    workflow: &WorkflowSpec,
    model: Option<&str>,
    requested_stage: Option<&str>,
) -> Result<(), String> {
    let mut run = initial.clone();
    if run.status != RunStatus::Running {
        return Ok(());
    }
    let stage = if let Some(stage_id) = requested_stage {
        workflow
            .stages
            .iter()
            .find(|stage| stage.id == stage_id)
            .ok_or_else(|| format!("unknown stage: {stage_id}"))?
    } else {
        match next_stage(workflow, &run.completed_stages) {
            Some(stage) => stage,
            None => {
                run.status = RunStatus::Completed;
                run.stage = None;
                run.updated_at = now();
                save_run(dir, &run)
                    .await
                    .map_err(|error| error.to_string())?;
                return Ok(());
            }
        }
    };
    run.stage = Some(stage.id.clone());
    run.updated_at = now();
    save_run(dir, &run)
        .await
        .map_err(|error| error.to_string())?;

    let session_id = run
        .session_id
        .clone()
        .ok_or_else(|| "run has no session".to_string())?;
    let prompt = stage_prompt(&run, stage, dir, artifacts);
    let model = model.map(|id| ModelSelection {
        id: id.to_string(),
        params: None,
    });
    sessions::send_message(
        state,
        &run.project_id,
        &session_id,
        &SendMessageRequest {
            text: prompt,
            model,
            images: Vec::new(),
        },
    )
    .await
    .map_err(|error| error.to_string())?;

    refresh_artifacts(&mut run, artifacts).await;
    check_outputs(&run, stage, artifacts)?;

    let human_gate = stage
        .gate
        .as_ref()
        .and_then(|id| workflow.gates.iter().find(|gate| &gate.id == id))
        .map(|gate| is_human(workflow, &gate.approver))
        .unwrap_or(false);

    if human_gate {
        run.status = RunStatus::WaitingGate;
        run.stage = Some(stage.id.clone());
        run.updated_at = now();
        save_run(dir, &run)
            .await
            .map_err(|error| error.to_string())?;
        return Ok(());
    }

    if !run.completed_stages.iter().any(|id| id == &stage.id) {
        run.completed_stages.push(stage.id.clone());
    }
    run.status = RunStatus::Created;
    run.stage = None;
    run.review_notes = None;
    run.updated_at = now();
    save_run(dir, &run)
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}
fn next_stage<'a>(workflow: &'a WorkflowSpec, completed: &[String]) -> Option<&'a StageSpec> {
    workflow.stages.iter().find(|stage| {
        !completed.iter().any(|id| id == &stage.id)
            && stage
                .depends_on
                .iter()
                .all(|dependency| completed.iter().any(|id| id == dependency))
    })
}

fn is_human(workflow: &WorkflowSpec, role_id: &str) -> bool {
    workflow
        .roles
        .iter()
        .find(|role| role.id == role_id)
        .map(|role| role.kind == RoleKind::Human)
        .unwrap_or(false)
}

fn check_outputs(run: &WorkflowRun, stage: &StageSpec, dir: &Path) -> Result<(), String> {
    for id in &stage.outputs {
        let artifact = run
            .artifact(id)
            .ok_or_else(|| format!("stage {} declares unknown artifact {id}", stage.id))?;
        let path = dir.join(&artifact.path);
        if artifact.is_dir() {
            if !path.is_dir() {
                return Err(format!(
                    "stage {} did not produce directory {}",
                    stage.id, artifact.path
                ));
            }
        } else if !path.is_file() {
            return Err(format!(
                "stage {} did not produce artifact {}",
                stage.id, artifact.path
            ));
        }
    }
    Ok(())
}

fn stage_prompt(
    run: &WorkflowRun,
    stage: &StageSpec,
    state_dir: &Path,
    artifact_dir: &Path,
) -> String {
    let project_root = artifact_dir.parent().unwrap_or(artifact_dir);
    let artifact_root = relative_path(project_root, artifact_dir);
    let state_root = relative_path(project_root, state_dir);
    let artifacts = run
        .artifacts
        .iter()
        .map(|artifact| {
            format!(
                "- {}: {}/{} ({})",
                artifact.id,
                artifact_root,
                artifact.path,
                artifact.status.as_str()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let outputs = stage
        .outputs
        .iter()
        .filter_map(|id| run.artifact(id))
        .map(|artifact| format!("- {} -> {}/{}", artifact.id, artifact_root, artifact.path))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "你正在执行学习助手的生产阶段「{name}」({id})。\n\n\
工件目录（相对项目根，Git 跟踪）：`{artifact_root}/`\n\
运行状态目录（不要写入工件）：`{state_root}/`\n\n\
本次运行输入：\n{input}\n\n\
当前工件：\n{artifacts}\n\n\
本阶段输出：\n{outputs}\n\n\
阶段说明：\n{prompt}\n\n\
要求：\n\
- 只把本阶段输出写入工件目录；不要写入运行状态目录，也不要覆盖无关工件。\n\
- 使用现有 agent 能力完成任务；需要委派时使用 tasks/dispatch，由协调者自动选择执行者。\n\
- 不要跳过门禁，不要修改已锁定的工件。\n\
- 完成后回复简短总结，并确认输出文件存在。\n",
        name = stage.name,
        id = stage.id,
        artifact_root = artifact_root,
        state_root = state_root,
        input = run.input,
        artifacts = artifacts,
        outputs = outputs,
        prompt = stage
            .prompt
            .as_deref()
            .unwrap_or("按阶段名称完成本阶段工作。"),
    )
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

async fn mark_failed(dir: &Path, run_id: &str, message: &str) {
    if let Ok(mut run) = load_run(dir, run_id).await {
        run.status = RunStatus::Failed;
        run.error = Some(message.to_string());
        run.stage = None;
        run.updated_at = now();
        let _ = save_run(dir, &run).await;
    }
}

fn select_workflow(run: &WorkflowRun) -> Result<WorkflowSpec, ApiError> {
    let workflows = crate::AssistantApp.workflows();
    let workflow = run
        .workflow_id
        .as_ref()
        .and_then(|id| workflows.iter().find(|workflow| &workflow.id == id))
        .cloned()
        .or_else(|| workflows.first().cloned());
    workflow.ok_or_else(|| ApiError::InvalidRequest("the assistant has no workflow".into()))
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

async fn load_for_project(
    state: &ServerState,
    pid: &str,
    run_id: &str,
) -> Result<(zoo_store::Project, PathBuf, WorkflowRun), ApiError> {
    let project = state
        .project(&ProjectId::from(pid))
        .await
        .ok_or(ApiError::ProjectNotFound)?;
    let dir = run_dir(&project.path, run_id);
    let mut run = load_run(&dir, run_id).await?;
    let mut changed = false;
    for spec in crate::AssistantApp.artifact_types() {
        if run.artifact(&spec.id).is_some() {
            continue;
        }
        run.artifacts.push(RunArtifact {
            id: spec.id.clone(),
            title: spec.title,
            path: spec.path.unwrap_or(spec.id),
            status: ArtifactStatus::Empty,
            size: 0,
            updated_at: None,
        });
        changed = true;
    }
    if changed {
        save_run(&dir, &run).await?;
    }
    Ok((project, dir, run))
}

async fn load_run(dir: &Path, run_id: &str) -> Result<WorkflowRun, ApiError> {
    let path = dir.join("run.json");
    let text = tokio::fs::read_to_string(&path)
        .await
        .map_err(|_| ApiError::FileNotFound(format!("run {run_id}")))?;
    serde_json::from_str(&text)
        .map_err(|error| ApiError::Internal(format!("invalid run {run_id}: {error}")))
}

async fn refresh_artifacts(run: &mut WorkflowRun, dir: &Path) {
    let now = chrono::Utc::now().to_rfc3339();
    for artifact in &mut run.artifacts {
        if artifact.is_dir() {
            continue;
        }
        let path = dir.join(&artifact.path);
        if let Ok(meta) = tokio::fs::metadata(&path).await {
            if meta.is_file() {
                artifact.status = ArtifactStatus::Draft;
                artifact.size = meta.len();
                artifact.updated_at = Some(now.clone());
            }
        }
    }
    run.updated_at = now;
}

async fn save_run(dir: &Path, run: &WorkflowRun) -> Result<(), ApiError> {
    let text =
        serde_json::to_string_pretty(run).map_err(|error| ApiError::Internal(error.to_string()))?;
    tokio::fs::write(dir.join("run.json"), text)
        .await
        .map_err(internal)
}

fn to_run(run: WorkflowRun) -> Run {
    Run {
        id: run.id,
        project_id: run.project_id,
        workflow_id: run.workflow_id,
        stage: run.stage,
        session_id: run.session_id,
        completed_stages: run.completed_stages,
        status: run.status.as_str().to_string(),
        input: run.input,
        error: run.error,
        review_notes: run.review_notes,
        created_at: run.created_at,
        updated_at: run.updated_at,
        artifacts: run
            .artifacts
            .into_iter()
            .map(|artifact| RunArtifactInfo {
                id: artifact.id,
                title: artifact.title,
                path: artifact.path,
                status: artifact.status.as_str().to_string(),
                size: artifact.size,
                updated_at: artifact.updated_at,
            })
            .collect(),
    }
}

fn internal(error: std::io::Error) -> ApiError {
    ApiError::Internal(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{ArtifactSpec, GateSpec, RoleSpec, StageSpec, WorkflowSpec};

    fn stage(id: &str, depends_on: &[&str], outputs: &[&str]) -> StageSpec {
        StageSpec {
            id: id.into(),
            name: id.into(),
            role: "worker".into(),
            requires: vec![],
            depends_on: depends_on.iter().map(|s| s.to_string()).collect(),
            inputs: vec![],
            outputs: outputs.iter().map(|s| s.to_string()).collect(),
            gate: None,
            foreach: None,
            prompt: None,
        }
    }

    fn workflow() -> WorkflowSpec {
        WorkflowSpec {
            id: "flow".into(),
            name: "Flow".into(),
            roles: vec![
                RoleSpec {
                    id: "worker".into(),
                    name: "Worker".into(),
                    kind: RoleKind::Agent,
                    capabilities: vec!["file_write".into()],
                    can_approve: vec![],
                },
                RoleSpec {
                    id: "producer".into(),
                    name: "Producer".into(),
                    kind: RoleKind::Human,
                    capabilities: vec![],
                    can_approve: vec!["a".into()],
                },
            ],
            gates: vec![GateSpec {
                id: "gate".into(),
                name: "Gate".into(),
                approver: "producer".into(),
                on_reject: None,
            }],
            stages: vec![
                stage("a", &[], &["artifact"]),
                stage("b", &["a"], &["artifact"]),
                stage("c", &["b"], &[]),
            ],
        }
    }

    #[test]
    fn sanitizes_upload_directories() {
        let path = sanitize_relative_dir("manuscripts/first draft").unwrap();
        assert_eq!(
            path.to_string_lossy().replace('\\', "/"),
            "manuscripts/first_draft"
        );
        let chinese = sanitize_relative_dir("中文资料/原稿").unwrap();
        assert_eq!(
            chinese.to_string_lossy().replace('\\', "/"),
            "中文资料/原稿"
        );
        assert!(sanitize_relative_dir("../outside").is_err());
        assert!(sanitize_relative_dir("a/../../b").is_err());
        assert!(sanitize_relative_dir("../../").is_err());
    }

    #[test]
    fn artifact_workspace_stays_out_of_runtime_state() {
        let root = Path::new("project");
        let artifacts = artifact_dir(root);
        assert_eq!(artifacts, root.join("assistant"));
        assert_ne!(artifacts, run_dir(root, "run"));
    }

    #[test]
    fn picks_the_next_ready_stage() {
        let flow = workflow();
        assert_eq!(next_stage(&flow, &[]).unwrap().id, "a");
        assert_eq!(next_stage(&flow, &["a".into()]).unwrap().id, "b");
        assert_eq!(
            next_stage(&flow, &["a".into(), "b".into()]).unwrap().id,
            "c"
        );
        assert!(next_stage(&flow, &["a".into(), "b".into(), "c".into()]).is_none());
    }

    #[test]
    fn resolves_human_roles() {
        let flow = workflow();
        assert!(is_human(&flow, "producer"));
        assert!(!is_human(&flow, "worker"));
        assert!(!is_human(&flow, "missing"));
    }

    #[test]
    fn checks_stage_outputs() {
        let dir = std::env::temp_dir().join(format!(
            "zoo-assistant-runs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let specs = vec![ArtifactSpec {
            id: "artifact".into(),
            title: "Artifact".into(),
            path: Some("artifact.json".into()),
            schema: None,
            version: 1,
        }];
        let run = WorkflowRun::new("run", "project", serde_json::json!({}), &specs);
        let stage = stage("a", &[], &["artifact"]);
        assert!(check_outputs(&run, &stage, &dir).is_err());
        std::fs::write(dir.join("artifact.json"), "{}").unwrap();
        assert!(check_outputs(&run, &stage, &dir).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

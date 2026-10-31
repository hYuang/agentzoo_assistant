//! Workflow run persistence.
//!
//! A run is one execution of the app workflow. The host keeps
//! runtime state (under `.zoo_agent`) separate from canonical artifacts so the
//! latter can be committed to Git:
//!
//! ```text
//! <project>/.zoo_agent/plugins/<plugin>/runs/<run>/run.json   runtime state
//! <project>/.zoo_agent/plugins/<plugin>/assets/               private material
//! <project>/<plugin>/<artifact path>                          tracked artifacts
//! ```
//!
//! Plugins declare artifact ids and paths; the host creates the run, stores
//! artifact content, and exposes the catalog to the clients. Domain semantics
//! stay in the plugin.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::workflow::{
    is_workflow_id, ArtifactStatus, WorkflowApp, WorkflowRun, RoleKind, RunArtifact, RunStatus,
    StageSpec, WorkflowSpec,
};
use axum::http::HeaderMap;
use axum::response::Response;
use base64::Engine;
use uuid::Uuid;
use zoo_shared::api::{
    ApiRequest, CreatePluginDirectoryRequest, CreatePluginDirectoryResponse,
    CreatePluginRunRequest, CreateSessionRequest, ModelSelection, PluginArtifactContent,
    PluginArtifactInfo, PluginRunInfo, PluginRunsResponse, RejectPluginRunRequest,
    SendMessageRequest, StartPluginRunRequest, UploadFileRequest, UploadFileResponse,
    WritePluginArtifactRequest,
};
use zoo_shared::message::ProjectId;

use zoo_server::service::packs::safe_join;
use zoo_server::service::{files, sessions, ServerState};
use zoo_server::ApiError;

/// The runtime state directory for a project/plugin/run.
pub fn run_dir(project_root: &Path, plugin_id: &str, run_id: &str) -> PathBuf {
    project_root
        .join(".zoo_agent")
        .join("plugins")
        .join(plugin_id)
        .join("runs")
        .join(run_id)
}

/// The canonical, Git-trackable artifact directory. One plugin owns one
/// workspace per project; runtime metadata stays in [`run_dir`].
pub fn artifact_dir(project_root: &Path, plugin_id: &str) -> PathBuf {
    project_root.join(plugin_id)
}

/// The plugin's private material root, kept under `.zoo_agent` alongside the
/// run state so the framework and plugins share one directory convention:
/// `<project>/.zoo_agent/plugins/<id>/assets/`.
pub fn plugin_assets_dir(project_root: &Path, plugin_id: &str) -> PathBuf {
    project_root
        .join(".zoo_agent")
        .join("plugins")
        .join(plugin_id)
        .join("assets")
}

/// The plugin's upload root: `<project>/.zoo_agent/plugins/<id>/assets/uploads/`.
pub fn plugin_uploads_dir(project_root: &Path, plugin_id: &str) -> PathBuf {
    plugin_assets_dir(project_root, plugin_id).join("uploads")
}

/// Create a run from a plugin entry. The run starts empty; stages write its
/// artifacts through [`write_artifact`].
pub async fn create_run(
    state: &ServerState,
    pid: &str,
    plugin_id: &str,
    body: &CreatePluginRunRequest,
) -> Result<PluginRunInfo, ApiError> {
    if !is_workflow_id(plugin_id) {
        return Err(ApiError::InvalidRequest(format!(
            "invalid plugin id: {plugin_id}"
        )));
    }
    body.validate()?;
    let project = state
        .project(&ProjectId::from(pid))
        .await
        .ok_or(ApiError::ProjectNotFound)?;
    let plugin = plugin(state, plugin_id).await?;
    let entry_id = resolve_entry(plugin.as_ref(), body.entry_id.as_deref())?;
    let mut run = WorkflowRun::new(
        Uuid::new_v4().to_string(),
        plugin_id,
        pid,
        entry_id,
        body.input.clone(),
        &plugin.artifact_types(),
    );
    run.workflow_id = plugin
        .workflows()
        .first()
        .map(|workflow| workflow.id.clone());
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
    let dir = run_dir(&project.path, plugin_id, &run.id);
    let artifacts = artifact_dir(&project.path, plugin_id);
    tokio::fs::create_dir_all(&dir).await.map_err(internal)?;
    tokio::fs::create_dir_all(&artifacts)
        .await
        .map_err(internal)?;
    save_run(&dir, &run).await?;
    refresh_artifacts(&mut run, &artifacts).await;
    save_run(&dir, &run).await?;
    Ok(to_info(run))
}

/// List a plugin's runs in one project, most recently updated first.
pub async fn list_runs(
    state: &ServerState,
    pid: &str,
    plugin_id: &str,
) -> Result<PluginRunsResponse, ApiError> {
    if !is_workflow_id(plugin_id) {
        return Err(ApiError::InvalidRequest(format!(
            "invalid plugin id: {plugin_id}"
        )));
    }
    let project = state
        .project(&ProjectId::from(pid))
        .await
        .ok_or(ApiError::ProjectNotFound)?;
    let _ = plugin(state, plugin_id).await?;
    let runs_dir = project
        .path
        .join(".zoo_agent")
        .join("plugins")
        .join(plugin_id)
        .join("runs");
    let mut runs = Vec::new();
    let mut entries = match tokio::fs::read_dir(&runs_dir).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PluginRunsResponse { runs });
        }
        Err(error) => return Err(internal(error)),
    };
    while let Some(entry) = entries.next_entry().await.map_err(internal)? {
        if !entry.file_type().await.map_err(internal)?.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        if let Ok(run) = load_run(&entry.path(), &id).await {
            runs.push(to_info(run));
        }
    }
    runs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(PluginRunsResponse { runs })
}

/// Load one run.
pub async fn get_run(
    state: &ServerState,
    pid: &str,
    plugin_id: &str,
    run_id: &str,
) -> Result<PluginRunInfo, ApiError> {
    let (_, _, run) = load_for_project(state, pid, plugin_id, run_id).await?;
    Ok(to_info(run))
}

/// Read one artifact. Text is returned as UTF-8; binary is base64-encoded.
pub async fn read_artifact(
    state: &ServerState,
    pid: &str,
    plugin_id: &str,
    run_id: &str,
    artifact_id: &str,
) -> Result<PluginArtifactContent, ApiError> {
    let (project, _, run) = load_for_project(state, pid, plugin_id, run_id).await?;
    let artifacts = artifact_dir(&project.path, plugin_id);
    let artifact = run
        .artifact(artifact_id)
        .ok_or_else(|| ApiError::PluginArtifactNotFound(artifact_id.to_string()))?;
    if artifact.is_dir() {
        return Err(ApiError::InvalidRequest(format!(
            "artifact {artifact_id} is a directory"
        )));
    }
    let path = safe_join(&artifacts, &artifact.path)
        .ok_or_else(|| ApiError::InvalidRequest("unsafe artifact path".into()))?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| ApiError::FileNotFound(artifact.path.clone()))?;
    match String::from_utf8(bytes.clone()) {
        Ok(content) => Ok(PluginArtifactContent {
            content,
            encoding: "utf-8".into(),
        }),
        Err(_) => Ok(PluginArtifactContent {
            content: base64::engine::general_purpose::STANDARD.encode(bytes),
            encoding: "base64".into(),
        }),
    }
}

/// Write one artifact and return the updated run.
pub async fn write_artifact(
    state: &ServerState,
    pid: &str,
    plugin_id: &str,
    run_id: &str,
    artifact_id: &str,
    body: &WritePluginArtifactRequest,
) -> Result<PluginRunInfo, ApiError> {
    body.validate()?;
    let (project, dir, mut run) = load_for_project(state, pid, plugin_id, run_id).await?;
    let artifacts = artifact_dir(&project.path, plugin_id);
    let artifact = run
        .artifact(artifact_id)
        .cloned()
        .ok_or_else(|| ApiError::PluginArtifactNotFound(artifact_id.to_string()))?;
    if artifact.is_dir() {
        return Err(ApiError::InvalidRequest(format!(
            "artifact {artifact_id} is a directory"
        )));
    }
    let path = safe_join(&artifacts, &artifact.path)
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
    Ok(to_info(run))
}

/// Upload a user-provided source file into the plugin's private material root
/// at `<project>/.zoo_agent/plugins/<id>/assets/uploads/`.
pub async fn upload_file(
    state: &ServerState,
    pid: &str,
    plugin_id: &str,
    run_id: &str,
    body: &UploadFileRequest,
) -> Result<UploadFileResponse, ApiError> {
    body.validate()?;
    let (project, _, _) = load_for_project(state, pid, plugin_id, run_id).await?;
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
    let dir = plugin_uploads_dir(&project.path, plugin_id).join(&relative_dir);
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

/// Create one directory under the plugin's upload root. Empty and parent
/// (`..`) segments are rejected so the path cannot escape `assets/uploads/`.
pub async fn create_directory(
    state: &ServerState,
    pid: &str,
    plugin_id: &str,
    run_id: &str,
    body: &CreatePluginDirectoryRequest,
) -> Result<CreatePluginDirectoryResponse, ApiError> {
    body.validate()?;
    let (project, _, _) = load_for_project(state, pid, plugin_id, run_id).await?;
    let relative = sanitize_relative_dir(&body.path)?;
    if relative.as_os_str().is_empty() {
        return Err(ApiError::InvalidRequest("empty directory path".into()));
    }
    let dir = plugin_uploads_dir(&project.path, plugin_id).join(&relative);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|error| ApiError::Internal(format!("failed to create directory: {error}")))?;
    Ok(CreatePluginDirectoryResponse {
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
    plugin_id: &str,
    run_id: &str,
    rel: &str,
) -> Result<(), ApiError> {
    if rel.trim().is_empty() {
        return Err(ApiError::InvalidRequest("empty upload path".into()));
    }
    let (project, _, _) = load_for_project(state, pid, plugin_id, run_id).await?;
    let base = plugin_uploads_dir(&project.path, plugin_id);
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
    plugin_id: &str,
    run_id: &str,
    rel: &str,
    headers: &HeaderMap,
) -> Result<Response, ApiError> {
    let (project, _, _) = load_for_project(state, pid, plugin_id, run_id).await?;
    let base = plugin_uploads_dir(&project.path, plugin_id);
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
    plugin_id: &str,
    run_id: &str,
    body: &StartPluginRunRequest,
) -> Result<PluginRunInfo, ApiError> {
    body.validate()?;
    let (project, dir, mut run) = load_for_project(state, pid, plugin_id, run_id).await?;
    let artifacts = artifact_dir(&project.path, plugin_id);
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
    let plugin = plugin(state, plugin_id).await?;
    let workflow = select_workflow(plugin.as_ref(), &run)?;
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
    Ok(to_info(run))
}

/// Approve the stage waiting at a human gate. It becomes completed, but the
/// next stage is not started until the user asks for it.
pub async fn approve_run(
    state: &Arc<ServerState>,
    pid: &str,
    plugin_id: &str,
    run_id: &str,
    body: &StartPluginRunRequest,
) -> Result<PluginRunInfo, ApiError> {
    body.validate()?;
    let (_, dir, mut run) = load_for_project(state, pid, plugin_id, run_id).await?;
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
    Ok(to_info(run))
}

/// Reject the current stage review. The stage stays incomplete so the next
/// generate re-runs it with the feedback.
pub async fn reject_run(
    state: &ServerState,
    pid: &str,
    plugin_id: &str,
    run_id: &str,
    body: &RejectPluginRunRequest,
) -> Result<PluginRunInfo, ApiError> {
    body.validate()?;
    let (_, dir, mut run) = load_for_project(state, pid, plugin_id, run_id).await?;
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
    Ok(to_info(run))
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
                plugin = %initial.plugin_id,
                run = %initial.id,
                error = %error,
                "plugin stage failed"
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
    let plugin_id = &run.plugin_id;
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
        "你正在执行 `{plugin_id}` 插件的生产阶段「{name}」({id})。\n\n\
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
        plugin_id = plugin_id,
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

fn select_workflow(plugin: &dyn WorkflowApp, run: &WorkflowRun) -> Result<WorkflowSpec, ApiError> {
    let workflows = plugin.workflows();
    let workflow = run
        .workflow_id
        .as_ref()
        .and_then(|id| workflows.iter().find(|workflow| &workflow.id == id))
        .cloned()
        .or_else(|| workflows.first().cloned());
    workflow.ok_or_else(|| {
        ApiError::InvalidRequest(format!("plugin {} has no workflow", plugin.manifest().id))
    })
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

async fn plugin(_state: &ServerState, id: &str) -> Result<Arc<dyn WorkflowApp>, ApiError> {
    if id == "assistant" {
        Ok(Arc::new(crate::AssistantApp))
    } else {
        Err(ApiError::PluginNotFound(id.to_string()))
    }
}

fn resolve_entry(plugin: &dyn WorkflowApp, requested: Option<&str>) -> Result<Option<String>, ApiError> {
    let entries = plugin.entries();
    match requested.map(str::trim).filter(|id| !id.is_empty()) {
        Some(id) => {
            if entries.iter().any(|entry| entry.id == id) {
                Ok(Some(id.to_string()))
            } else {
                Err(ApiError::InvalidRequest(format!(
                    "unknown plugin entry: {id}"
                )))
            }
        }
        None => Ok(entries.first().map(|entry| entry.id.clone())),
    }
}

async fn load_for_project(
    state: &ServerState,
    pid: &str,
    plugin_id: &str,
    run_id: &str,
) -> Result<(zoo_store::Project, PathBuf, WorkflowRun), ApiError> {
    if !is_workflow_id(plugin_id) || !is_workflow_id(run_id) {
        return Err(ApiError::InvalidRequest("invalid plugin or run id".into()));
    }
    let plugin = plugin(state, plugin_id).await?;
    let project = state
        .project(&ProjectId::from(pid))
        .await
        .ok_or(ApiError::ProjectNotFound)?;
    let dir = run_dir(&project.path, plugin_id, run_id);
    let mut run = load_run(&dir, run_id).await?;
    let mut changed = false;
    for spec in plugin.artifact_types() {
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
        .map_err(|_| ApiError::PluginRunNotFound)?;
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

fn to_info(run: WorkflowRun) -> PluginRunInfo {
    PluginRunInfo {
        id: run.id,
        plugin_id: run.plugin_id,
        project_id: run.project_id,
        entry_id: run.entry_id,
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
            .map(|artifact| PluginArtifactInfo {
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
        let artifacts = artifact_dir(root, "audiobook");
        assert_eq!(artifacts, root.join("audiobook"));
        assert_ne!(artifacts, run_dir(root, "audiobook", "run"));
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
            "zoo-plugin-runs-{}-{}",
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
        let run = WorkflowRun::new(
            "run",
            "plugin",
            "project",
            None,
            serde_json::json!({}),
            &specs,
        );
        let stage = stage("a", &[], &["artifact"]);
        assert!(check_outputs(&run, &stage, &dir).is_err());
        std::fs::write(dir.join("artifact.json"), "{}").unwrap();
        assert!(check_outputs(&run, &stage, &dir).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

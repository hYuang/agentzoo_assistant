use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::Json;
use zoo_shared::api::{
    ApiResult, CreatePluginDirectoryRequest, CreatePluginDirectoryResponse, CreatePluginRunRequest,
    PluginArtifactContent, PluginRunInfo, PluginRunsResponse, RejectPluginRunRequest,
    StartPluginRunRequest, UploadFileRequest, UploadFileResponse, WritePluginArtifactRequest,
};

use zoo_server::service::ServerState;
use zoo_server::ApiError;

use crate::runs;

pub(crate) async fn create(
    State(state): State<Arc<ServerState>>,
    Path((pid, plugin)): Path<(String, String)>,
    Json(body): Json<CreatePluginRunRequest>,
) -> Result<Json<ApiResult<PluginRunInfo>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::create_run(&state, &pid, &plugin, &body).await?,
    )))
}

pub(crate) async fn list(
    State(state): State<Arc<ServerState>>,
    Path((pid, plugin)): Path<(String, String)>,
) -> Result<Json<ApiResult<PluginRunsResponse>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::list_runs(&state, &pid, &plugin).await?,
    )))
}

pub(crate) async fn get(
    State(state): State<Arc<ServerState>>,
    Path((pid, plugin, run)): Path<(String, String, String)>,
) -> Result<Json<ApiResult<PluginRunInfo>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::get_run(&state, &pid, &plugin, &run).await?,
    )))
}

pub(crate) async fn start(
    State(state): State<Arc<ServerState>>,
    Path((pid, plugin, run)): Path<(String, String, String)>,
    Json(body): Json<StartPluginRunRequest>,
) -> Result<Json<ApiResult<PluginRunInfo>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::start_run(&state, &pid, &plugin, &run, &body).await?,
    )))
}

pub(crate) async fn approve(
    State(state): State<Arc<ServerState>>,
    Path((pid, plugin, run)): Path<(String, String, String)>,
    Json(body): Json<StartPluginRunRequest>,
) -> Result<Json<ApiResult<PluginRunInfo>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::approve_run(&state, &pid, &plugin, &run, &body).await?,
    )))
}

pub(crate) async fn reject(
    State(state): State<Arc<ServerState>>,
    Path((pid, plugin, run)): Path<(String, String, String)>,
    Json(body): Json<RejectPluginRunRequest>,
) -> Result<Json<ApiResult<PluginRunInfo>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::reject_run(&state, &pid, &plugin, &run, &body).await?,
    )))
}

pub(crate) async fn read_artifact(
    State(state): State<Arc<ServerState>>,
    Path((pid, plugin, run, artifact)): Path<(String, String, String, String)>,
) -> Result<Json<ApiResult<PluginArtifactContent>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::read_artifact(&state, &pid, &plugin, &run, &artifact).await?,
    )))
}

pub(crate) async fn write_artifact(
    State(state): State<Arc<ServerState>>,
    Path((pid, plugin, run, artifact)): Path<(String, String, String, String)>,
    Json(body): Json<WritePluginArtifactRequest>,
) -> Result<Json<ApiResult<PluginRunInfo>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::write_artifact(&state, &pid, &plugin, &run, &artifact, &body).await?,
    )))
}

pub(crate) async fn upload(
    State(state): State<Arc<ServerState>>,
    Path((pid, plugin, run)): Path<(String, String, String)>,
    Json(body): Json<UploadFileRequest>,
) -> Result<Json<ApiResult<UploadFileResponse>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::upload_file(&state, &pid, &plugin, &run, &body).await?,
    )))
}

pub(crate) async fn create_directory(
    State(state): State<Arc<ServerState>>,
    Path((pid, plugin, run)): Path<(String, String, String)>,
    Json(body): Json<CreatePluginDirectoryRequest>,
) -> Result<Json<ApiResult<CreatePluginDirectoryResponse>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::create_directory(&state, &pid, &plugin, &run, &body).await?,
    )))
}

pub(crate) async fn serve_upload(
    State(state): State<Arc<ServerState>>,
    Path((pid, plugin, run, rel)): Path<(String, String, String, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    runs::serve_upload(&state, &pid, &plugin, &run, &rel, &headers).await
}

pub(crate) async fn delete_upload(
    State(state): State<Arc<ServerState>>,
    Path((pid, plugin, run, rel)): Path<(String, String, String, String)>,
) -> Result<Json<ApiResult<()>>, ApiError> {
    runs::delete_upload(&state, &pid, &plugin, &run, &rel).await?;
    Ok(Json(ApiResult::ok(())))
}

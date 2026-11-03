use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::Json;
use zoo_server::service::ServerState;
use zoo_server::ApiError;
use zoo_shared::api::ApiResult;

use crate::api::{
    ArtifactContent, CreateDirectory, CreateDirectoryResponse, CreateRun, RejectRun, Run, Runs,
    StartRun, UploadFile, UploadFileResponse, WriteArtifact,
};
use crate::runs;

pub(crate) async fn create(
    State(state): State<Arc<ServerState>>,
    Path(pid): Path<String>,
    Json(body): Json<CreateRun>,
) -> Result<Json<ApiResult<Run>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::create_run(&state, &pid, &body).await?,
    )))
}

pub(crate) async fn list(
    State(state): State<Arc<ServerState>>,
    Path(pid): Path<String>,
) -> Result<Json<ApiResult<Runs>>, ApiError> {
    Ok(Json(ApiResult::ok(runs::list_runs(&state, &pid).await?)))
}

pub(crate) async fn get(
    State(state): State<Arc<ServerState>>,
    Path((pid, run)): Path<(String, String)>,
) -> Result<Json<ApiResult<Run>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::get_run(&state, &pid, &run).await?,
    )))
}

pub(crate) async fn start(
    State(state): State<Arc<ServerState>>,
    Path((pid, run)): Path<(String, String)>,
    Json(body): Json<StartRun>,
) -> Result<Json<ApiResult<Run>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::start_run(&state, &pid, &run, &body).await?,
    )))
}

pub(crate) async fn approve(
    State(state): State<Arc<ServerState>>,
    Path((pid, run)): Path<(String, String)>,
    Json(body): Json<StartRun>,
) -> Result<Json<ApiResult<Run>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::approve_run(&state, &pid, &run, &body).await?,
    )))
}

pub(crate) async fn reject(
    State(state): State<Arc<ServerState>>,
    Path((pid, run)): Path<(String, String)>,
    Json(body): Json<RejectRun>,
) -> Result<Json<ApiResult<Run>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::reject_run(&state, &pid, &run, &body).await?,
    )))
}

pub(crate) async fn read_artifact(
    State(state): State<Arc<ServerState>>,
    Path((pid, run, artifact)): Path<(String, String, String)>,
) -> Result<Json<ApiResult<ArtifactContent>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::read_artifact(&state, &pid, &run, &artifact).await?,
    )))
}

pub(crate) async fn write_artifact(
    State(state): State<Arc<ServerState>>,
    Path((pid, run, artifact)): Path<(String, String, String)>,
    Json(body): Json<WriteArtifact>,
) -> Result<Json<ApiResult<Run>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::write_artifact(&state, &pid, &run, &artifact, &body).await?,
    )))
}

pub(crate) async fn upload(
    State(state): State<Arc<ServerState>>,
    Path((pid, run)): Path<(String, String)>,
    Json(body): Json<UploadFile>,
) -> Result<Json<ApiResult<UploadFileResponse>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::upload_file(&state, &pid, &run, &body).await?,
    )))
}

pub(crate) async fn create_directory(
    State(state): State<Arc<ServerState>>,
    Path((pid, run)): Path<(String, String)>,
    Json(body): Json<CreateDirectory>,
) -> Result<Json<ApiResult<CreateDirectoryResponse>>, ApiError> {
    Ok(Json(ApiResult::ok(
        runs::create_directory(&state, &pid, &run, &body).await?,
    )))
}

pub(crate) async fn serve_upload(
    State(state): State<Arc<ServerState>>,
    Path((pid, run, rel)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    runs::serve_upload(&state, &pid, &run, &rel, &headers).await
}

pub(crate) async fn delete_upload(
    State(state): State<Arc<ServerState>>,
    Path((pid, run, rel)): Path<(String, String, String)>,
) -> Result<Json<ApiResult<()>>, ApiError> {
    runs::delete_upload(&state, &pid, &run, &rel).await?;
    Ok(Json(ApiResult::ok(())))
}

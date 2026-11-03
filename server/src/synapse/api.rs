use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde_json::Value;
use zoo_server::service::ServerState;
use zoo_server::ApiError;
use zoo_shared::api::ApiResult;

use crate::synapse::{self, LoginRequest, PublishOptions, SynapseConfig};

pub(crate) async fn get(
    State(state): State<Arc<ServerState>>,
    Path((pid, run)): Path<(String, String)>,
) -> Result<Json<ApiResult<Value>>, ApiError> {
    Ok(Json(ApiResult::ok(
        synapse::status(&state, &pid, &run).await?,
    )))
}

pub(crate) async fn publish(
    State(state): State<Arc<ServerState>>,
    Path((pid, run)): Path<(String, String)>,
    Json(options): Json<PublishOptions>,
) -> Result<Json<ApiResult<Value>>, ApiError> {
    Ok(Json(ApiResult::ok(
        synapse::publish(&state, &pid, &run, options).await?,
    )))
}

pub(crate) async fn login(
    State(state): State<Arc<ServerState>>,
    Path((pid, run)): Path<(String, String)>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<ApiResult<Value>>, ApiError> {
    let config = SynapseConfig::from_env();
    Ok(Json(ApiResult::ok(
        synapse::login(&state, &pid, &run, &config, request).await?,
    )))
}

pub(crate) async fn logout(
    State(state): State<Arc<ServerState>>,
    Path((pid, run)): Path<(String, String)>,
) -> Json<ApiResult<Value>> {
    Json(ApiResult::ok(synapse::logout(&state, &pid, &run).await))
}

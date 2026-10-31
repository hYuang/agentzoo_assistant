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
    Path((pid, plugin, run)): Path<(String, String, String)>,
) -> Result<Json<ApiResult<Value>>, ApiError> {
    Ok(Json(ApiResult::ok(
        synapse::status(&state, &pid, &plugin, &run).await?,
    )))
}

pub(crate) async fn publish(
    State(state): State<Arc<ServerState>>,
    Path((pid, plugin, run)): Path<(String, String, String)>,
    Json(options): Json<PublishOptions>,
) -> Result<Json<ApiResult<Value>>, ApiError> {
    Ok(Json(ApiResult::ok(
        synapse::publish(&state, &pid, &plugin, &run, options).await?,
    )))
}

pub(crate) async fn login(
    Json(request): Json<LoginRequest>,
) -> Result<Json<ApiResult<Value>>, ApiError> {
    let config = SynapseConfig::from_env();
    Ok(Json(ApiResult::ok(synapse::login(&config, request).await?)))
}

pub(crate) async fn logout() -> Json<ApiResult<Value>> {
    Json(ApiResult::ok(synapse::logout()))
}

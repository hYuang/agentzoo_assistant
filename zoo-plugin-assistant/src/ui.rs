use axum::extract::Path;
use axum::http::header;
use axum::response::{IntoResponse, Response};

use crate::sdk::{is_plugin_id, Plugin};
use zoo_server::ApiError;

/// `GET /plugins/{name}/ui/{*path}` — serve this app's static UI.
pub(crate) async fn ui_asset(
    Path((name, path)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    if !is_plugin_id(&name) || name != "assistant" {
        return Err(ApiError::PluginNotFound(name));
    }
    let path = path.trim_start_matches('/');
    let bytes = crate::AssistantPlugin
        .ui_asset(path)
        .ok_or_else(|| ApiError::FileNotFound(path.to_string()))?;
    let content_type = match path.rsplit('.').next() {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    };
    Ok(([(header::CONTENT_TYPE, content_type)], bytes).into_response())
}

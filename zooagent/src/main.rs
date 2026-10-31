//! Product binary: composes the framework with the learning assistant plugin.

use std::sync::Arc;

use zoo_plugins::{Plugin, PluginCtx};

fn builtin(_base: &PluginCtx) -> Vec<Arc<dyn Plugin>> {
    vec![Arc::new(zoo_plugin_assistant::AssistantPlugin)]
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    zoo_server::app::run(builtin).await
}

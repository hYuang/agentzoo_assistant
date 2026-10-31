//! Product binary: composes the framework with the selected vertical plugins.

use std::sync::Arc;

use zoo_plugins::{Plugin, PluginCtx};

fn builtin(_base: &PluginCtx) -> Vec<Arc<dyn Plugin>> {
    #[allow(unused_mut)]
    let mut plugins: Vec<Arc<dyn Plugin>> = Vec::new();
    #[cfg(feature = "assistant")]
    plugins.push(Arc::new(zoo_plugin_assistant::AssistantPlugin));
    #[cfg(feature = "audiobook")]
    plugins.push(Arc::new(zoo_plugin_audiobook::AudiobookPlugin));
    #[cfg(feature = "observability")]
    plugins.push(Arc::new(
        zoo_plugin_observability::ObservabilityPlugin::with_context(Arc::new(
            _base.for_plugin("observability"),
        )),
    ));
    #[cfg(feature = "paper")]
    plugins.push(Arc::new(zoo_plugin_paper::PaperPlugin));
    plugins
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    zoo_server::app::run(builtin).await
}

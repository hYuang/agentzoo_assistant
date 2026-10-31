//! Product binary: composes the framework with the learning assistant app.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("ZOO_WEB_DIST").is_none() {
        std::env::set_var("ZOO_WEB_DIST", "web/dist");
    }
    zoo_bootstrap::app::run(zoo_agent_server::extension()).await
}

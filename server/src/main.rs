//! Product binary: composes the framework with the learning assistant app.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    zoo_bootstrap::app::run(zoo_agent_server::app()).await
}

//! Product binary: composes the framework with the learning assistant app.

use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("ZOO_WEB_DIST").is_none() {
        if let Some(dir) = find_host_dist() {
            std::env::set_var("ZOO_WEB_DIST", dir);
        }
    }
    zoo_bootstrap::app::run(zoo_agent_server::extension()).await
}

fn find_host_dist() -> Option<PathBuf> {
    let mut candidates = vec![PathBuf::from("web/dist")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            candidates.push(exe_dir.join("web"));
            if let Some(target) = exe_dir.parent() {
                if let Some(repo) = target.parent() {
                    candidates.push(repo.join("web/dist"));
                }
            }
        }
    }
    candidates
        .into_iter()
        .find(|dir| dir.join("index.html").is_file())
}

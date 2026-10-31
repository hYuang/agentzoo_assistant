/// Errors raised while registering or running a plugin.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("invalid plugin manifest: {0}")]
    InvalidManifest(String),
    #[error("plugin already registered: {0}")]
    Duplicate(String),
    #[error("unknown plugin: {0}")]
    Unknown(String),
    #[error("plugin {plugin}: {message}")]
    Lifecycle { plugin: String, message: String },
    #[error("plugin {plugin} left running after a failed start: {message}")]
    Start { plugin: String, message: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}

impl PluginError {
    pub fn lifecycle(plugin: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Lifecycle {
            plugin: plugin.into(),
            message: message.into(),
        }
    }
}

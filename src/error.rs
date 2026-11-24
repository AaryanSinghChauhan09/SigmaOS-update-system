use thiserror::Error;

#[derive(Error, Debug)]
pub enum UpdateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("TOML error: {0}")]
    Toml(#[from] toml::ser::Error),
    
    #[error("TOML error: {0}")]
    TomlDe(#[from] toml::de::Error),
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Update error: {0}")]
    Update(String),
    
    #[error("Config error: {0}")]
    Config(String),
    
    #[error("State error: {0}")]
    State(String),
    
    #[error("Invalid version: {0}")]
    InvalidVersion(String),
    
    #[error("No previous version")]
    NoPreviousVersion,
    
    #[error("Install failed")]
    InstallFailed,
    
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
}

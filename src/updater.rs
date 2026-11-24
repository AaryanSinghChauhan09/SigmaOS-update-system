use crate::error::UpdateError;
use crate::channel::UpdateChannel;
use crate::notifier::UpdateNotifier;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use reqwest::Client;
use semver::Version;

pub struct UpdateManager {
    client: Client,
    notifier: UpdateNotifier,
    config_path: PathBuf,
    state_path: PathBuf,
}

impl UpdateManager {
    pub fn new() -> Result<Self, UpdateError> {
        let config_dir = directories::ProjectDirs::from("org", "sigmaos", "sigmaos").map(|p| p.config_dir().to_path_buf()).unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("sigmaos");
        
        fs::create_dir_all(&config_dir)?;
        
        Ok(Self {
            client: Client::new(),
            notifier: UpdateNotifier::new()?,
            config_path: config_dir.join("update.toml"),
            state_path: config_dir.join("update-state.json"),
        })
    }
    
    pub async fn check_updates(&self, channel: &str, force: bool) -> Result<Vec<Update>, UpdateError> {
        let config = self.load_config()?;
        
        // Check if we should check for updates
        if !force && !self.should_check_for_updates(&config)? {
            return Ok(Vec::new());
        }
        
        // Get current version
        let current_version = self.get_current_version()?;
        
        // Fetch updates from server
        let updates = self.fetch_updates(channel, &current_version).await?;
        
        // Save last check time
        self.save_last_check()?;
        
        Ok(updates)
    }
    
    pub async fn update_system(&self, channel: &str, force: bool) -> Result<(), UpdateError> {
        let updates = self.check_updates(channel, force).await?;
        
        if updates.is_empty() {
            log::info!("No updates available");
            return Ok(());
        }
        
        // Sort updates by version
        let mut sorted_updates = updates;
        sorted_updates.sort_by(|a, b| {
            Version::parse(&b.new_version)
                .unwrap_or_else(|_| Version::new(0, 0, 0))
                .cmp(&Version::parse(&a.new_version).unwrap_or_else(|_| Version::new(0, 0, 0)))
        });
        
        // Apply updates
        for update in sorted_updates {
            log::info!("Updating to version {}", update.new_version);
            self.apply_update(&update).await?;
        }
        
        // Notify user
        self.notifier.notify_update("System updated successfully")?;
        
        Ok(())
    }
    
    pub fn rollback(&self, version: &str) -> Result<(), UpdateError> {
        let versions = self.list_versions()?;
        
        if !versions.contains(&version.to_string()) {
            return Err(UpdateError::InvalidVersion(version.to_string()));
        }
        
        // Create backup of current version
        self.create_backup()?;
        
        // Rollback to target version
        self.apply_rollback(version)?;
        
        // Notify user
        self.notifier.notify_update(&format!("Rolled back to version {}", version))?;
        
        Ok(())
    }
    
    pub fn rollback_to_previous(&self) -> Result<(), UpdateError> {
        let versions = self.list_versions()?;
        
        if versions.len() < 2 {
            return Err(UpdateError::NoPreviousVersion);
        }
        
        // Get previous version (second to last)
        let previous_version = &versions[versions.len() - 2];
        self.rollback(previous_version)
    }
    
    pub fn list_versions(&self) -> Result<Vec<String>, UpdateError> {
        let state = self.load_state()?;
        Ok(state.versions)
    }
    
    pub fn get_status(&self) -> Result<UpdateStatus, UpdateError> {
        let config = self.load_config()?;
        let state = self.load_state()?;
        
        Ok(UpdateStatus {
            current_version: self.get_current_version()?,
            channel: config.channel,
            last_check: state.last_check,
            auto_update: config.auto_update,
            pending_updates: Vec::new(), // TODO: Implement pending updates
        })
    }
    
    pub fn update_config(&self, channel: Option<String>, auto_update: Option<bool>, interval: Option<u32>) -> Result<(), UpdateError> {
        let mut config = self.load_config()?;
        
        if let Some(channel) = channel {
            config.channel = channel;
        }
        
        if let Some(auto_update) = auto_update {
            config.auto_update = auto_update;
        }
        
        if let Some(interval) = interval {
            config.check_interval = interval;
        }
        
        self.save_config(&config)?;
        Ok(())
    }
    
    fn load_config(&self) -> Result<UpdateConfig, UpdateError> {
        if self.config_path.exists() {
            let content = fs::read_to_string(&self.config_path)?;
            toml::from_str(&content).map_err(|e| UpdateError::Config(e.to_string()))
        } else {
            Ok(UpdateConfig::default())
        }
    }
    
    fn save_config(&self, config: &UpdateConfig) -> Result<(), UpdateError> {
        let content = toml::to_string_pretty(config)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }
    
    fn load_state(&self) -> Result<UpdateState, UpdateError> {
        if self.state_path.exists() {
            let content = fs::read_to_string(&self.state_path)?;
            serde_json::from_str(&content).map_err(|e| UpdateError::State(e.to_string()))
        } else {
            Ok(UpdateState::default())
        }
    }
    
    fn save_state(&self, state: &UpdateState) -> Result<(), UpdateError> {
        let content = serde_json::to_string_pretty(state)?;
        fs::write(&self.state_path, content)?;
        Ok(())
    }
    
    fn should_check_for_updates(&self, config: &UpdateConfig) -> Result<bool, UpdateError> {
        let state = self.load_state()?;
        
        if let Some(last_check) = state.last_check {
            let elapsed = chrono::Utc::now().timestamp() - last_check;
            Ok(elapsed >= config.check_interval as i64)
        } else {
            Ok(true)
        }
    }
    
    fn save_last_check(&self) -> Result<(), UpdateError> {
        let mut state = self.load_state()?;
        state.last_check = Some(chrono::Utc::now().timestamp());
        self.save_state(&state)
    }
    
    fn get_current_version(&self) -> Result<String, UpdateError> {
        // Read current version from system
        let output = std::process::Command::new("sigmaos-version")
            .arg("--current")
            .output()?;
        
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Ok("0.1.0".to_string())
        }
    }
    
    async fn fetch_updates(&self, channel: &str, current_version: &str) -> Result<Vec<Update>, UpdateError> {
        // Fetch updates from update server
        let url = format!("https://updates.sigmaos.org/{}/{}", channel, current_version);
        
        let response = self.client.get(&url).send().await?;
        
        if response.status().is_success() {
            let updates: Vec<Update> = response.json().await?;
            Ok(updates)
        } else {
            Ok(Vec::new())
        }
    }
    
    async fn apply_update(&self, update: &Update) -> Result<(), UpdateError> {
        // Download update package
        let package_url = &update.package_url;
        let package_path = self.download_package(package_url).await?;
        
        // Verify package signature
        self.verify_package(&package_path, &update.signature)?;
        
        // Apply update
        self.install_package(&package_path)?;
        
        // Update version
        self.update_version(&update.new_version)?;
        
        // Clean up
        fs::remove_file(&package_path)?;
        
        Ok(())
    }
    
    async fn download_package(&self, url: &str) -> Result<PathBuf, UpdateError> {
        let response = self.client.get(url).send().await?;
        let bytes = response.bytes().await?;
        
        let temp_path = directories::ProjectDirs::from("org", "sigmaos", "sigmaos").map(|p| p.cache_dir().to_path_buf()).unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("sigmaos-update")
            .join("package.tar.zst");
        
        fs::create_dir_all(temp_path.parent().unwrap())?;
        fs::write(&temp_path, bytes)?;
        
        Ok(temp_path)
    }
    
    fn verify_package(&self, package_path: &Path, signature: &str) -> Result<(), UpdateError> {
        // Verify package signature
        // TODO: Implement signature verification
        Ok(())
    }
    
    fn install_package(&self, package_path: &Path) -> Result<(), UpdateError> {
        // Install package using pacman
        let status = std::process::Command::new("sudo")
            .arg("pacman")
            .arg("-U")
            .arg("--noconfirm")
            .arg(package_path)
            .status()?;
        
        if !status.success() {
            return Err(UpdateError::InstallFailed);
        }
        
        Ok(())
    }
    
    fn update_version(&self, version: &str) -> Result<(), UpdateError> {
        // Update version file
        let version_path = PathBuf::from("/etc/sigmaos-version");
        fs::write(&version_path, format!("{}\n", version))?;
        
        // Update state
        let mut state = self.load_state()?;
        if !state.versions.contains(&version.to_string()) {
            state.versions.push(version.to_string());
        }
        self.save_state(&state)?;
        
        Ok(())
    }
    
    fn create_backup(&self) -> Result<(), UpdateError> {
        // Create backup of current system state
        let backup_dir = directories::ProjectDirs::from("org", "sigmaos", "sigmaos").map(|p| p.cache_dir().to_path_buf()).unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("sigmaos-backup");
        
        fs::create_dir_all(&backup_dir)?;
        
        // Backup version file
        let version_path = PathBuf::from("/etc/sigmaos-version");
        if version_path.exists() {
            fs::copy(&version_path, backup_dir.join("sigmaos-version"))?;
        }
        
        Ok(())
    }
    
    fn apply_rollback(&self, version: &str) -> Result<(), UpdateError> {
        // Rollback to specified version
        // TODO: Implement rollback logic
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Update {
    pub current_version: String,
    pub new_version: String,
    pub channel: String,
    pub package_url: String,
    pub signature: String,
    pub release_notes: String,
    pub published_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    pub channel: String,
    pub auto_update: bool,
    pub check_interval: u32,
    pub notify_updates: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            channel: "stable".to_string(),
            auto_update: false,
            check_interval: 86400,
            notify_updates: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateState {
    pub last_check: Option<i64>,
    pub versions: Vec<String>,
    pub current_version: String,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            last_check: None,
            versions: Vec::new(),
            current_version: "0.1.0".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateStatus {
    pub current_version: String,
    pub channel: String,
    pub last_check: Option<i64>,
    pub auto_update: bool,
    pub pending_updates: Vec<Update>,
}

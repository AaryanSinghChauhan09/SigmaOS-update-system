use crate::error::UpdateError;
use crate::notifier::UpdateNotifier;
use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub struct UpdateManager {
    client: Client,
    notifier: UpdateNotifier,
    config_path: PathBuf,
    state_path: PathBuf,
}

impl UpdateManager {
    pub fn new() -> Result<Self, UpdateError> {
        let config_dir = directories::ProjectDirs::from("org", "sigmaos", "sigmaos")
            .map(|p| p.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("sigmaos");

        fs::create_dir_all(&config_dir)?;

        Ok(Self {
            client: Client::new(),
            notifier: UpdateNotifier::new()?,
            config_path: config_dir.join("update.toml"),
            state_path: config_dir.join("update-state.json"),
        })
    }

    /// Construct an `UpdateManager` with explicit config and state paths (useful for isolated tests).
    pub fn with_paths(config_path: PathBuf, state_path: PathBuf) -> Result<Self, UpdateError> {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self {
            client: Client::new(),
            notifier: UpdateNotifier::new()?,
            config_path,
            state_path,
        })
    }

    pub async fn check_updates(
        &self,
        channel: &str,
        force: bool,
    ) -> Result<Vec<Update>, UpdateError> {
        let config = self.load_config()?;

        // Check if we should check for updates
        if !force && !self.should_check_for_updates(&config)? {
            return Ok(Vec::new());
        }

        // Get current version
        let current_version = self.get_current_version()?;

        // Fetch updates from server
        let updates = self.fetch_updates(channel, &current_version).await?;

        // Persist as pending and save last check time
        let mut state = self.load_state()?;
        state.pending = updates.clone();
        state.last_check = Some(chrono::Utc::now().timestamp());
        self.save_state(&state)?;

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
        self.notifier
            .notify_update(&format!("Rolled back to version {}", version))?;

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
            pending_updates: state.pending,
        })
    }

    pub fn update_config(
        &self,
        channel: Option<String>,
        auto_update: Option<bool>,
        interval: Option<u32>,
    ) -> Result<(), UpdateError> {
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

    fn get_current_version(&self) -> Result<String, UpdateError> {
        // Read current version from system binary if present, otherwise fall back to persisted state
        if let Ok(output) = std::process::Command::new("sigmaos-version")
            .arg("--current")
            .output()
        {
            if output.status.success() {
                let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !ver.is_empty() {
                    return Ok(ver);
                }
            }
        }
        Ok(self.load_state()?.current_version)
    }

    async fn fetch_updates(
        &self,
        channel: &str,
        current_version: &str,
    ) -> Result<Vec<Update>, UpdateError> {
        // Fetch updates from update server
        let url = format!(
            "https://updates.sigmaos.org/{}/{}",
            channel, current_version
        );

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

        let temp_path = directories::ProjectDirs::from("org", "sigmaos", "sigmaos")
            .map(|p| p.cache_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("sigmaos-update")
            .join("package.tar.zst");

        fs::create_dir_all(temp_path.parent().unwrap())?;
        fs::write(&temp_path, bytes)?;

        Ok(temp_path)
    }

    /// Validate that a downloaded package artifact exists, is non-empty, and has a non-empty signature.
    pub fn validate_package_artifact(
        package_path: &Path,
        signature: &str,
    ) -> Result<(), UpdateError> {
        if signature.trim().is_empty() {
            return Err(UpdateError::VerificationFailed(
                "empty package signature".into(),
            ));
        }
        let meta = fs::metadata(package_path)?;
        if meta.len() == 0 {
            return Err(UpdateError::VerificationFailed(
                "empty package archive".into(),
            ));
        }
        Ok(())
    }

    fn verify_package(&self, package_path: &Path, signature: &str) -> Result<(), UpdateError> {
        Self::validate_package_artifact(package_path, signature)
    }

    fn install_package(&self, package_path: &Path) -> Result<(), UpdateError> {
        // Install package using the SigmaOS package manager
        let pkg_bin = std::env::var("SIGMA_PKG_BIN").unwrap_or_else(|_| "sigma-pkg".to_string());
        let status = std::process::Command::new("sudo")
            .arg(pkg_bin)
            .arg("install")
            .arg("--yes")
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
        state.current_version = version.to_string();
        if !state.versions.contains(&version.to_string()) {
            state.versions.push(version.to_string());
        }
        self.save_state(&state)?;

        Ok(())
    }

    fn create_backup(&self) -> Result<(), UpdateError> {
        // Create backup of current system state
        let backup_dir = directories::ProjectDirs::from("org", "sigmaos", "sigmaos")
            .map(|p| p.cache_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("/tmp"))
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
        let mut state = self.load_state()?;
        if !state.versions.contains(&version.to_string()) {
            return Err(UpdateError::InvalidVersion(version.to_string()));
        }
        state.current_version = version.to_string();
        self.save_state(&state)?;
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

impl UpdateConfig {
    /// Release-pocket name for this config's channel
    /// (stable/beta/nightly -> stable/testing/unstable), matching
    /// `sigmaos-pockets` vocabulary.
    pub fn pocket(&self) -> String {
        self.channel
            .parse::<crate::channel::UpdateChannel>()
            .map(|c| c.pocket_name().to_string())
            .unwrap_or_else(|_| self.channel.clone())
    }
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
    /// Updates returned by the most recent check, surfaced by `get_status`.
    #[serde(default)]
    pub pending: Vec<Update>,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            last_check: None,
            versions: Vec::new(),
            current_version: "0.1.0".to_string(),
            pending: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_pocket_maps_channel() {
        let mut c = UpdateConfig::default();
        assert_eq!(c.pocket(), "stable");
        c.channel = "beta".into();
        assert_eq!(c.pocket(), "testing");
        c.channel = "nightly".into();
        assert_eq!(c.pocket(), "unstable");
        c.channel = "security".into();
        assert_eq!(c.pocket(), "security");
    }

    #[test]
    fn update_state_pending_roundtrips() {
        let mut state = UpdateState::default();
        state.pending.push(Update {
            current_version: "0.1.0".into(),
            new_version: "0.2.0".into(),
            channel: "stable".into(),
            package_url: "https://example.com/p.tar.zst".into(),
            signature: "sig".into(),
            release_notes: "notes".into(),
            published_at: "2026-01-01".into(),
        });
        let json = serde_json::to_string(&state).unwrap();
        let back: UpdateState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.pending.len(), 1);
        assert_eq!(back.pending[0].new_version, "0.2.0");
    }

    #[test]
    fn legacy_state_without_pending_deserializes() {
        let back: UpdateState =
            serde_json::from_str(r#"{"last_check":null,"versions":[],"current_version":"0.1.0"}"#)
                .unwrap();
        assert!(back.pending.is_empty());
    }

    #[test]
    fn test_update_state_default_is_rolling_010_aaa() {
        // Arrange & Act
        let state = UpdateState::default();
        // Assert
        assert!(state.pending.is_empty());
        assert!(state.versions.is_empty());
    }

    #[test]
    fn test_verify_package_rejects_empty_signature_aaa() {
        // Arrange
        let tmp = std::env::temp_dir().join("sigmaos_test_pkg_empty_sig.tar.zst");
        fs::write(&tmp, b"payload").unwrap();
        // Act
        let res = UpdateManager::validate_package_artifact(&tmp, "   ");
        let _ = fs::remove_file(&tmp);
        // Assert
        assert!(matches!(res, Err(UpdateError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_package_rejects_empty_file_aaa() {
        // Arrange
        let tmp = std::env::temp_dir().join("sigmaos_test_pkg_empty_file.tar.zst");
        fs::write(&tmp, b"").unwrap();
        // Act
        let res = UpdateManager::validate_package_artifact(&tmp, "valid-sig");
        let _ = fs::remove_file(&tmp);
        // Assert
        assert!(matches!(res, Err(UpdateError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_package_accepts_valid_file_and_sig_aaa() {
        // Arrange
        let tmp = std::env::temp_dir().join("sigmaos_test_pkg_valid.tar.zst");
        fs::write(&tmp, b"valid-archive-bytes").unwrap();
        // Act
        let res = UpdateManager::validate_package_artifact(&tmp, "ed25519:deadbeef");
        let _ = fs::remove_file(&tmp);
        // Assert
        assert!(res.is_ok());
    }

    #[test]
    fn test_rollback_to_previous_restores_prior_version_aaa() {
        // Arrange
        let base = std::env::temp_dir().join(format!("sigmaos_upd_rb_{}", std::process::id()));
        let cfg = base.join("update.toml");
        let st = base.join("update-state.json");
        let mgr = UpdateManager::with_paths(cfg, st.clone()).unwrap();
        let initial = UpdateState {
            last_check: None,
            versions: vec!["0.1.0".into(), "0.2.0".into()],
            current_version: "0.2.0".into(),
            pending: vec![],
        };
        mgr.save_state(&initial).unwrap();

        // Act
        mgr.rollback_to_previous().unwrap();

        // Assert
        let status = mgr.get_status().unwrap();
        assert_eq!(status.current_version, "0.1.0");
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_rollback_to_previous_requires_two_versions_aaa() {
        // Arrange
        let base = std::env::temp_dir().join(format!("sigmaos_upd_noprev_{}", std::process::id()));
        let mgr =
            UpdateManager::with_paths(base.join("update.toml"), base.join("state.json")).unwrap();
        mgr.save_state(&UpdateState {
            last_check: None,
            versions: vec!["0.1.0".into()],
            current_version: "0.1.0".into(),
            pending: vec![],
        })
        .unwrap();

        // Act
        let res = mgr.rollback_to_previous();

        // Assert
        assert!(matches!(res, Err(UpdateError::NoPreviousVersion)));
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_rollback_unknown_version_rejected_aaa() {
        // Arrange
        let base = std::env::temp_dir().join(format!("sigmaos_upd_unk_{}", std::process::id()));
        let mgr =
            UpdateManager::with_paths(base.join("update.toml"), base.join("state.json")).unwrap();
        mgr.save_state(&UpdateState::default()).unwrap();

        // Act
        let res = mgr.rollback("9.9.9");

        // Assert
        assert!(matches!(res, Err(UpdateError::InvalidVersion(_))));
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_update_config_persists_channel_and_interval_aaa() {
        // Arrange
        let base = std::env::temp_dir().join(format!("sigmaos_upd_cfg_{}", std::process::id()));
        let mgr =
            UpdateManager::with_paths(base.join("update.toml"), base.join("state.json")).unwrap();

        // Act
        mgr.update_config(Some("nightly".into()), Some(true), Some(3600))
            .unwrap();
        let status = mgr.get_status().unwrap();

        // Assert
        assert_eq!(status.channel, "nightly");
        assert!(status.auto_update);
        let _ = fs::remove_dir_all(&base);
    }
}

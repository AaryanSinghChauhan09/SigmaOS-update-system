use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateChannel {
    Stable,
    Beta,
    Nightly,
    Custom(String),
}

impl UpdateChannel {
    pub fn as_str(&self) -> &str {
        match self {
            UpdateChannel::Stable => "stable",
            UpdateChannel::Beta => "beta",
            UpdateChannel::Nightly => "nightly",
            UpdateChannel::Custom(s) => s,
        }
    }

    /// Map the channel onto a release-pocket name used by
    /// `sigmaos-pockets` (Debian-style: stable/testing/unstable).
    pub fn pocket_name(&self) -> &str {
        match self {
            UpdateChannel::Stable => "stable",
            UpdateChannel::Beta => "testing",
            UpdateChannel::Nightly => "unstable",
            UpdateChannel::Custom(s) => s,
        }
    }
}

impl std::str::FromStr for UpdateChannel {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "stable" => UpdateChannel::Stable,
            "beta" => UpdateChannel::Beta,
            "nightly" => UpdateChannel::Nightly,
            _ => UpdateChannel::Custom(s.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pocket_name_maps_channels() {
        assert_eq!(UpdateChannel::Stable.pocket_name(), "stable");
        assert_eq!(UpdateChannel::Beta.pocket_name(), "testing");
        assert_eq!(UpdateChannel::Nightly.pocket_name(), "unstable");
        assert_eq!(
            UpdateChannel::Custom("security".into()).pocket_name(),
            "security"
        );
        // as_str keeps its own vocabulary
        assert_eq!(UpdateChannel::Beta.as_str(), "beta");
    }
}

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

//! Global server configuration
//!
//! These types are [deserialized][serde::Deserialize] from a [`toml`] configuration file.

use std::{net::SocketAddr, path::Path};

use eyre::WrapErr;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    pub limits: LimitsConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    pub network: NetworkConfig,
}

impl Config {
    /// Parse tohe config from a [`toml`] file.
    pub async fn from_file(path: impl AsRef<Path>) -> eyre::Result<Self> {
        let path = path.as_ref();
        let contents = tokio::fs::read_to_string(path)
            .await
            .wrap_err_with(|| format!("Could not read config file at {}", path.display()))?;
        let cfg = toml::from_str(&contents)
            .wrap_err_with(|| format!("Could not parse config file at {}", path.display()))?;
        Ok(cfg)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LimitsConfig {
    pub max_in_flight_msgs: usize,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_in_flight_msgs: 1024,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LoggingConfig {
    pub filter: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            filter: String::from("info"),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct NetworkConfig {
    pub bind_address: SocketAddr,
}

use std::{net::SocketAddr, path::Path};

use eyre::WrapErr;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub limits: LimitsConfig,
    pub logging: LoggingConfig,
    pub network: NetworkConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LimitsConfig {
    pub max_in_flight_msgs: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LoggingConfig {
    pub filter: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct NetworkConfig {
    pub bind_address: SocketAddr,
}

impl Config {
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

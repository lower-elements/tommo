//! Global server configuration
//!
//! These types are [deserialized][serde::Deserialize] from a [`toml`] configuration file.

use std::{net::SocketAddr, sync::Arc, path::Path};

use eyre::WrapErr;
use serde::Deserialize;
use tokio::{net::TcpListener, task::JoinHandle};

use crate::state::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    pub motd: Option<String>,
    #[serde(default)]
    pub database: deadpool_postgres::Config,
    #[serde(default)]
    pub limits: LimitsConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    pub listener: Vec<ListenerConfig>,
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

    pub fn listeners<'a>(&'a self, state: &'a Arc<State>) -> impl Iterator<Item = JoinHandle<eyre::Result<()>>> + 'a {
        self.listener.iter().map(|l| l.listen(Arc::clone(state)))
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
pub struct ListenerConfig {
    #[serde(default = "ListenerConfig::default_name")]
    pub name: String,
    pub bind: SocketAddr,
}

impl ListenerConfig {
    fn default_name() -> String {
        String::from("unnamed")
    }

pub fn listen(&self, state: Arc<State>) -> JoinHandle<eyre::Result<()>> {
    let bind_addr = self.bind;
    let name = self.name.clone();
    tokio::spawn(async move {
        let span = tracing::info_span!("listener", name = %name, %bind_addr);
        let _guard = span.enter();
    let listener = TcpListener::bind(bind_addr)
        .await
        .wrap_err_with(|| format!("Could not bind to address: {}", bind_addr))?;
    tracing::info!("Now listening");
    loop {
        match listener
            .accept()
            .await
            .wrap_err("Could not accept connection")
        {
            Ok((conn, addr)) => {
                // Create a new client-handler
                let handler = state.new_connection(conn);
                tokio::spawn(async move {
                    match handler.handle().await {
                        Ok(_) => tracing::info!(%addr, "Client disconnected"),
                        Err(e) => tracing::warn!(error = ?e, %addr, "Client error"),
                    }
                });
            }
            Err(e) => tracing::error!(error = ?e, "Failed to accept connection"),
        }
    }
    })
}
}

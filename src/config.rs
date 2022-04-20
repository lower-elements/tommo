//! Global server configuration
//!
//! These types are [deserialized][serde::Deserialize] from a [`toml`] configuration file.

use std::{net::SocketAddr, path::Path, sync::Arc};

use eyre::WrapErr;
use mlua::prelude::*;
use serde::Deserialize;
use tokio::{net::TcpListener, task::JoinHandle};

use crate::state::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub motd: Option<String>,
    #[serde(default)]
    pub database: deadpool_postgres::Config,
    #[serde(default)]
    pub limits: LimitsConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    pub listeners: Vec<ListenerConfig>,
}

impl Config {
    /// Parse a `Config` from a Lua file
    pub async fn from_lua(path: impl AsRef<Path>) -> eyre::Result<Self> {
        let path = path.as_ref();
        let contents = tokio::fs::read(path)
            .await
            .wrap_err_with(|| format!("Could not read config file at {}", path.display()))?;
        let lua = Lua::new();
        lua.load(&contents)
            .exec_async()
            .await
            .wrap_err_with(|| format!("Failed to run config file {}", path.display()))?;
        let val: LuaValue = lua
            .globals()
            .get("config")
            .wrap_err("Config does not contain a global `config' table")?;
        let cfg = lua.from_value(val).wrap_err("Invalid configuration")?;
        Ok(cfg)
    }

    pub fn listeners<'a>(
        &'a self,
        state: &'a Arc<State>,
    ) -> impl Iterator<Item = JoinHandle<eyre::Result<()>>> + 'a {
        self.listeners.iter().map(|l| l.listen(Arc::clone(state)))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitsConfig {
    #[serde(default = "LimitsConfig::default_max_in_flight_msgs")]
    pub max_in_flight_msgs: usize,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_in_flight_msgs: 1024,
        }
    }
}

impl LimitsConfig {
    fn default_max_in_flight_msgs() -> usize {
        Self::default().max_in_flight_msgs
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfig {
    #[serde(default = "LoggingConfig::default_filter")]
    pub filter: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            filter: String::from("info"),
        }
    }
}

impl LoggingConfig {
    fn default_filter() -> String {
        Self::default().filter
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
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

use std::sync::Arc;

use deadpool_postgres::{tokio_postgres::NoTls, Runtime};
use eyre::WrapErr;
use tokio::{net::TcpStream, sync::broadcast};

use crate::{config::Config, connection::Connection};

/// Global server state, responsible for creating new connection handlers, managing database
/// connections, and storing config values, among other things.
pub struct State {
    /// Server configuration. Private to make sure all accesses from outside that `State` are
    /// immutable.
    config: Config,
    /// A pool of database connections.
    db_pool: deadpool_postgres::Pool,
    /// The sending end of a channel that broadcasts to all clients. Public to allow for sending serverwide messages.
    pub global_tx: broadcast::Sender<String>,
}

impl State {
    /// Create a new state from the specified [`Config`].
    pub async fn new(config: Config) -> eyre::Result<Self> {
        let db_pool = config
            .database
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .wrap_err("Failed to connect to database")?;
        // Ensure we can actually connect
        let _ = db_pool
            .get()
            .await
            .wrap_err("Failed to connect to database")?;
        tracing::info!("Connected to database");

        let (global_tx, _) = broadcast::channel(config.limits.max_in_flight_msgs);
        Ok(Self {
            config,
            db_pool,
            global_tx,
        })
    }

    /// Get a reference to the [server configuration][crate::config::Config].
    #[inline]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Create a connection handler for a newly accepted connection.
    ///
    /// A connection handler is a [`Future`] that asynchronously handles the client's I/O, command
    /// processing, etc. It returns when the client has disconnected.
    pub fn new_connection(self: &Arc<Self>, socket: TcpStream) -> Arc<Connection> {
        Connection::new(socket, self.clone())
    }
}

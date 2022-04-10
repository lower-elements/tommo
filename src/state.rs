use std::{future::Future, net::SocketAddr, path::Path};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::broadcast::{self, error::RecvError},
};

use crate::config::Config;

/// Global server state, responsible for creating new connection handlers, managing database
/// connections, and storing config values, among other things.
#[derive(Debug)]
pub struct State {
    /// Server configuration. Private to make sure all accesses from outside that `State` are
    /// immutable.
    config: Config,
    /// The sending end of a channel that broadcasts to all clients. Public to allow for sending serverwide messages.
    pub global_tx: broadcast::Sender<String>,
}

impl State {
    /// Create a new state with the parameters specified in the config file at `config_path`.
    pub async fn new(config_path: &Path) -> eyre::Result<Self> {
        let config = Config::from_file(config_path).await?;
        let (global_tx, _rx) = broadcast::channel(config.limits.max_in_flight_msgs);
        Ok(Self { config, global_tx })
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
    #[tracing::instrument(name = "connection", skip(self, conn))]
    pub fn new_connection(
        &self,
        conn: TcpStream,
        addr: SocketAddr,
    ) -> impl Future<Output = eyre::Result<()>> {
        let (reader, writer) = conn.into_split();
        // Create handles to the broadcast channel for this client
        let global_tx = self.global_tx.clone();
        let global_rx = global_tx.subscribe();

        // This is the connection handler Future
        async move {
            tracing::info!("Client connected");

            // Handle received messages
            let send_handler = tokio::spawn(handle_send(writer, global_rx));

            // Handle sent messages
            let res = handle_recv(reader, global_tx).await;
            // If we reach this point, the connection is dead
            send_handler.abort();
            res
        }
    }
}

/// Send messages to the client when they're received over the broadcast channel.
#[tracing::instrument(level = "debug", skip_all)]
async fn handle_send(
    mut writer: OwnedWriteHalf,
    mut global_rx: broadcast::Receiver<String>,
) -> eyre::Result<()> {
    loop {
        match global_rx.recv().await {
            Ok(msg) => {
                writer.write_all(msg.as_bytes()).await?;
                writer.flush().await?;
            }
            Err(e) => match e {
                RecvError::Closed => return Ok(()), // Server shutting down
                RecvError::Lagged(by) => {
                    tracing::warn!(lagged_by = by, "Too many messages recieved")
                }
            },
        }
    }
}

/// Send messages down the channel when they're received from the client.
#[tracing::instrument(level = "debug", skip_all)]
async fn handle_recv(
    reader: OwnedReadHalf,
    global_tx: broadcast::Sender<String>,
) -> eyre::Result<()> {
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    loop {
        match reader.read_line(&mut line).await {
            Ok(n) if n == 0 => return Ok(()), // Client disconnected
            Ok(_) => {
                // Message received
                // The only possible error is when there are no receivers, which we can ignore
                global_tx.send(line.clone()).ok();
            }
            Err(e) => {
                tracing::warn!(error = ?e, "Could not read line from client");
                return Err(e.into());
            }
        }
        line.clear();
    }
}

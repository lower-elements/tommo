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

pub struct State {
    config: Config,
    global_tx: broadcast::Sender<String>,
    #[allow(dead_code)]
    global_rx: broadcast::Receiver<String>,
}

impl State {
    pub async fn new(config_path: &Path) -> eyre::Result<Self> {
        let config = Config::from_file(config_path).await?;
        let (global_tx, global_rx) = broadcast::channel(config.limits.max_in_flight_msgs);
        Ok(Self {
            config,
            global_tx,
            global_rx,
        })
    }

    #[inline]
    pub fn config(&self) -> &Config {
        &self.config
    }

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

        async move {
            tracing::info!("Client connected");

            // Handle received messages
            let send_handler = tokio::spawn(handle_send(writer, global_rx));

            // Handle sent messages
            let res = handle_recv(reader, global_tx).await;
            // If we reach this point, the connection id dead
            send_handler.abort();
            res
        }
    }
}

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

#[tracing::instrument(level = "debug", skip_all)]
async fn handle_recv(
    reader: OwnedReadHalf,
    global_tx: broadcast::Sender<String>,
) -> eyre::Result<()> {
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    loop {
        match reader.read_line(&mut line).await {
            Ok(n) if n == 0 => return Ok(()), // EOF
            Ok(_) => {
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

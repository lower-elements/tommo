use std::{future::Future, net::SocketAddr};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::broadcast::{self, error::RecvError},
};

const MAX_IN_FLIGHT_MSGS: usize = 1024;

pub struct State {
    global_tx: broadcast::Sender<String>,
    #[allow(dead_code)]
    global_rx: broadcast::Receiver<String>,
}

impl State {
    pub fn new() -> Self {
        let (global_tx, global_rx) = broadcast::channel(MAX_IN_FLIGHT_MSGS);
        Self {
            global_tx,
            global_rx,
        }
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

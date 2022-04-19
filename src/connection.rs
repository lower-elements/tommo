use std::sync::Arc;

use eyre::WrapErr;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    net::{tcp::{OwnedReadHalf, OwnedWriteHalf}, TcpStream},
    sync::broadcast::error::RecvError,
};

use crate::{
    state::State,
    util::{BufReadWrite, BroadcastReadWrite}
};

pub struct Connection {
    state: Arc<State>,
    conn: BufReadWrite<OwnedReadHalf, OwnedWriteHalf>,
    bcast: BroadcastReadWrite<String>,
}

impl Connection {
    pub fn new(socket: TcpStream, state: Arc<State>) -> Arc<Self> {
        let global_tx = state.global_tx.clone();
        Arc::new(Self {
            state,
            conn: BufReadWrite::buffered(socket),
            bcast: BroadcastReadWrite::from(global_tx),
        })
    }

    pub async fn handle(self: Arc<Self>) -> eyre::Result<()> {
        // Fail gracefully if we can't retrieve the peer address
        // On Linux at least, having read getpeername(2), I'm pretty sure any error we could
        // possibly receive here is fatal, but this may not be the case on other platforms.
        let span = match self.conn.read().await.get_ref().peer_addr() {
        Ok(addr) => tracing::info_span!("connection", %addr),
        Err(e) => tracing::info_span!("connection", addr = %"unknown", error = ?e),
        };
        let _guard = span.enter();
            tracing::info!("Client connected");

            // Write the MOTD
            if let Some(motd) = &self.state.config().motd {
                self.conn.with_tx(|mut tx| async move {
                    tx.write_all(motd.as_bytes()).await?;
                    if !motd.ends_with('\n') {
                        tx.write_all(b"\r\n").await?;
                    }
                    tx.flush().await?;
                    Ok::<_, std::io::Error>(())
                }).await.wrap_err("Error writing MOTD")?;
            }

            // Handle received messages
            let send_self = self.clone();
            let send_handler = tokio::spawn(send_self.handle_send());

            // Handle sent messages
            let res = self.handle_recv().await;
            // If we reach this point, the connection is dead
            send_handler.abort();
            res
    }

/// Send messages to the client when they're received over the broadcast channel.
#[tracing::instrument(level = "debug", skip_all)]
async fn handle_send(self: Arc<Self>) -> eyre::Result<()> {
    // We're the only code using this, so we can lock it indefinitely
    let mut global_rx = self.bcast.read().await;

    loop {
        match global_rx.recv().await {
            Ok(msg) => {
                let mut tx = self.conn.write().await;
                tx.write_all(msg.as_bytes()).await?;
                tx.flush().await?;
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
async fn handle_recv(self: Arc<Self>) -> eyre::Result<()> {
    // We're the only code using this, so we can lock it indefinitely
    let global_tx = self.bcast.write().await;
    let mut line = String::new();

    loop {
        match self.conn.read().await.read_line(&mut line).await {
            Ok(n) if n == 0 => return Ok(()), // Client disconnected
            Ok(_) => {
                // Message received
                // The only possible error is no receivers, which we can ignore
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
}

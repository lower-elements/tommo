use std::{future::Future, net::SocketAddr};

use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    net::{tcp::OwnedReadHalf, TcpStream},
    sync::broadcast::{self, error::RecvError},
};

const MAX_IN_FLIGHT_MSGS: usize = 1024;

pub struct State {
    global_tx: broadcast::Sender<String>,
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
        let (reader, mut writer) = conn.into_split();
        // Create handles to the broadcast channel for this client
        let global_tx = self.global_tx.clone();
        let mut global_rx = global_tx.subscribe();

        async move {
            tracing::info!("Client connected");

            // Handle sent messages
            tokio::spawn(handle_recv(reader, global_tx));

            // Handle received messages
            loop {
                match global_rx.recv().await {
                    Ok(msg) => {
                        writer.write_all(msg.as_bytes()).await?;
                        writer.flush().await?;
                    }
                    Err(e) => match e {
                        RecvError::Closed => break,
                        RecvError::Lagged(by) => {
                            tracing::warn!(lagged_by = by, "Too many messages recieved")
                        }
                    },
                }
            }
            Ok(())
        }
    }
}

#[tracing::instrument(skip_all)]
async fn handle_recv(
    reader: OwnedReadHalf,
    global_tx: broadcast::Sender<String>,
) -> eyre::Result<()> {
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    loop {
        match reader.read_line(&mut line).await {
            Ok(n) if n == 0 => break, // EOF
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
    Ok(())
}

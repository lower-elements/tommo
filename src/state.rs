use std::{future::Future, net::SocketAddr};

use tokio::{io::AsyncWriteExt, net::TcpStream, sync::broadcast::{self, error::RecvError}};

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
    pub fn new_connection(&self, mut conn: TcpStream, addr: SocketAddr) -> impl Future<Output = ()> {
        let global_tx = self.global_tx.clone();
        let mut global_rx = global_tx.subscribe();

        async move {
    tracing::info!( "Client connected");
    loop {
        match global_rx.recv().await {
            Ok(msg) => {
                conn.write_all(msg.as_bytes()).await.unwrap();
                conn.flush().await.unwrap();
            },
            Err(e) => {
                match e {
                RecvError::Closed => break,
                RecvError::Lagged(by) => tracing::warn!(lagged_by = by, "Too many messages recieved"),
            }
            },
        }
    }
        }
    }
}

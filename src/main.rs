use std::net::SocketAddr;

use eyre::WrapErr;
use tokio::{io::AsyncWriteExt, net::{TcpListener, TcpStream}};

const BIND_ADDR: &str = "0.0.0.0:7878";

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind(BIND_ADDR).await.wrap_err_with(|| format!("Could not bind to address: {}", BIND_ADDR))?;
    tracing::info!(addr = BIND_ADDR, "Now listening");
    loop {
        match listener.accept().await.wrap_err("Could not accept connection") {
            Ok((conn, addr)) => {
                tokio::spawn(handle_connection(conn, addr));
            },
            Err(e) => tracing::error!(error = ?e, "Failed to accept connection"),
        }
    }
}

#[tracing::instrument(name = "connection", skip(conn))]
async fn handle_connection(mut conn: TcpStream, addr: SocketAddr) {
    tracing::info!( "Client connected");
    conn.write_all(b"Hello, world!\n").await.unwrap();
    conn.flush().await.unwrap();
}

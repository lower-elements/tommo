use std::net::SocketAddr;

use eyre::WrapErr;
use tokio::{io::AsyncWriteExt, net::{TcpListener, TcpStream}};

const BIND_ADDR: &str = "0.0.0.0:7878";

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let listener = TcpListener::bind(BIND_ADDR).await.wrap_err_with(|| format!("Could not bind to address: {}", BIND_ADDR))?;
    loop {
        let (mut conn, addr) = listener.accept().await.wrap_err("Could not accept connection")?;
        tokio::spawn(handle_connection(conn, addr));
    }
}

async fn handle_connection(mut conn: TcpStream, addr: SocketAddr) {
    eprintln!("{} has connected", addr);
    conn.write_all(b"Hello, world!\n").await.unwrap();
    conn.flush().await.unwrap();
}

mod logging;
mod state;
use state::State;

use eyre::WrapErr;
use tokio::net::TcpListener;

const BIND_ADDR: &str = "0.0.0.0:7878";

#[tokio::main]
async fn main() -> eyre::Result<()> {
    logging::init();

    let state = State::new();

    let listener = TcpListener::bind(BIND_ADDR)
        .await
        .wrap_err_with(|| format!("Could not bind to address: {}", BIND_ADDR))?;
    tracing::info!(addr = BIND_ADDR, "Now listening");
    loop {
        match listener
            .accept()
            .await
            .wrap_err("Could not accept connection")
        {
            Ok((conn, addr)) => {
                // Create a new client-handler
                let handler = state.new_connection(conn, addr);
                tokio::spawn(async move {
                    match handler.await {
                        Ok(_) => tracing::info!(%addr, "Client disconnected"),
                        Err(e) => tracing::warn!(error = ?e, addr = %addr, "Client error"),
                    }
                });
            }
            Err(e) => tracing::error!(error = ?e, "Failed to accept connection"),
        }
    }
}

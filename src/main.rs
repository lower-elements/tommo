mod args;
mod config;
mod logging;
mod state;
use state::State;

use eyre::WrapErr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = args::parse();
    let state = State::new(&args.config).await?;

    let bind_addr = state.config().network.bind_address;
    let listener = TcpListener::bind(bind_addr)
        .await
        .wrap_err_with(|| format!("Could not bind to address: {}", bind_addr))?;
    tracing::info!(addr = %bind_addr, "Now listening");
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
                        Err(e) => tracing::warn!(error = ?e, %addr, "Client error"),
                    }
                });
            }
            Err(e) => tracing::error!(error = ?e, "Failed to accept connection"),
        }
    }
}

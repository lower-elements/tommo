mod args;
mod config;
mod connection;
use config::Config;
mod logging;
mod state;
use state::State;
mod util;

use std::sync::Arc;

use futures::stream::{self, StreamExt};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = args::parse();
    let config = Config::from_lua(&args.config).await?;

    // Initialize logging
    logging::init(&config.logging.filter);
    tracing::info!(version = %env!("CARGO_PKG_VERSION"), concat!("Starting ", env!("CARGO_PKG_NAME")));

    let state = Arc::new(State::new(config).await?);

    let listeners = state.config().listeners(&state);
    let len = listeners.len();
    let mut stream = stream::iter(listeners).buffer_unordered(len);
    while let Some(res) = stream.next().await {
        res??; // Proppogate errors
    }
    Ok(())
}

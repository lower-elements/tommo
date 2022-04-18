mod args;
mod config;
mod logging;
mod state;
use state::State;

use std::sync::Arc;

use futures::stream::{self, StreamExt};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = args::parse();
    let state = Arc::new(State::new(&args.config).await?);
    let listeners = state.config().listeners(&state);
    let len = state.config().listener.len();
    let mut stream = stream::iter(listeners).buffer_unordered(len);
    while let Some(res) = stream.next().await {
        res??; // Proppogate errors
    }
    Ok(())
}

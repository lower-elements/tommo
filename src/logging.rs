//!Logging with the [`tracing`] crate.

use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt, prelude::*};

/// Initialise the logging stack
pub fn init() {
    let subscriber = tracing_subscriber::Registry::default()
        .with(fmt::layer())
        .with(ErrorLayer::default());
    if let Err(e) = tracing::subscriber::set_global_default(subscriber) {
        eprintln!("Warning: Failed to set log handler: {}", e);
    }
    if let Err(e) = color_eyre::install() {
        tracing::warn!(error = %e, "Failed to install error / panic handler");
    }
}

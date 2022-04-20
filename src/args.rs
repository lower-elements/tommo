use std::path::PathBuf;

use clap::Parser;

/// Command-line arguments
#[derive(Parser, Debug)]
#[clap(about, version)]
pub struct Args {
    /// Path to the configuration Lua script
    #[clap(short, long, default_value = "config.lua")]
    pub config: PathBuf,
}

/// Parse command-line arguments from [`std::env::args_os`]. This function exists so we don't have
/// to import [`clap::Parser`] in main.rs.
pub fn parse() -> Args {
    Args::parse()
}

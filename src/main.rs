use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;

mod config;
mod plugin;
mod protocol;
mod server;

// Conditional compilation for optional features
#[cfg(feature = "audio-processing")]
mod audio;

#[cfg(feature = "discord")]
mod player;

#[cfg(feature = "discord")]
mod voice;

#[cfg(test)]
mod test_utils;

use config::LavalinkConfig;
use server::LavalinkServer;

#[derive(Parser)]
#[command(name = "lavalink-rust")]
#[command(about = "A standalone audio sending node for Discord, written in Rust")]
#[command(version)]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "application.yml")]
    config: PathBuf,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    init_tracing(args.verbose)?;

    info!("Starting Lavalink Rust v{}", env!("CARGO_PKG_VERSION"));
    info!("Loading configuration from: {}", args.config.display());

    // Load configuration
    let config = LavalinkConfig::load(&args.config).await?;

    // Print startup banner
    print_banner();

    // Create and start the server
    let server = LavalinkServer::new(config).await?;
    server.run().await?;

    Ok(())
}

fn init_tracing(verbose: bool) -> Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let filter = if verbose {
        EnvFilter::new("lavalink_rust=debug,info")
    } else {
        EnvFilter::new("lavalink_rust=info,warn")
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}

fn print_banner() {
    let banner = r#"
    ██╗      █████╗ ██╗   ██╗ █████╗ ██╗     ██╗███╗   ██╗██╗  ██╗    ██████╗ ██╗   ██╗███████╗████████╗
    ██║     ██╔══██╗██║   ██║██╔══██╗██║     ██║████╗  ██║██║ ██╔╝    ██╔══██╗██║   ██║██╔════╝╚══██╔══╝
    ██║     ███████║██║   ██║███████║██║     ██║██╔██╗ ██║█████╔╝     ██████╔╝██║   ██║███████╗   ██║
    ██║     ██╔══██║╚██╗ ██╔╝██╔══██║██║     ██║██║╚██╗██║██╔═██╗     ██╔══██╗██║   ██║╚════██║   ██║
    ███████╗██║  ██║ ╚████╔╝ ██║  ██║███████╗██║██║ ╚████║██║  ██╗    ██║  ██║╚██████╔╝███████║   ██║
    ╚══════╝╚═╝  ╚═╝  ╚═══╝  ╚═╝  ╚═╝╚══════╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝    ╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝
    "#;

    info!("{}", banner);
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("Powered by Rust 🦀");
}

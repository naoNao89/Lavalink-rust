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

    /// Enable verbose logging (info level)
    #[arg(short, long)]
    verbose: bool,

    /// Enable debug logging (debug level)
    #[arg(short, long)]
    debug: bool,

    /// Enable trace logging (trace level) - very verbose
    #[arg(short, long)]
    trace: bool,

    /// Custom log level (overrides verbose/debug/trace)
    #[arg(long, value_name = "LEVEL")]
    log_level: Option<String>,

    /// Output logs in JSON format
    #[arg(long)]
    json_logs: bool,

    /// Disable colored output
    #[arg(long)]
    no_color: bool,

    /// Show timestamps in logs
    #[arg(long)]
    timestamps: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    init_tracing(&args)?;

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

fn init_tracing(args: &Args) -> Result<()> {
    use tracing_subscriber::{
        fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
    };

    // Determine log level priority: custom > trace > debug > verbose > default
    let log_level = if let Some(ref level) = args.log_level {
        level.clone()
    } else if args.trace {
        "trace".to_string()
    } else if args.debug {
        "debug".to_string()
    } else if args.verbose {
        "info".to_string()
    } else {
        "info".to_string()
    };

    // Create filter, respecting RUST_LOG environment variable if set
    let filter = if let Ok(env_filter) = std::env::var("RUST_LOG") {
        // If RUST_LOG is set, use it but allow CLI override
        if args.log_level.is_some() || args.trace || args.debug || args.verbose {
            // CLI args override RUST_LOG
            create_filter(&log_level)
        } else {
            // Use RUST_LOG as-is
            EnvFilter::new(env_filter)
        }
    } else {
        create_filter(&log_level)
    };

    // Create formatter layer
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(args.trace) // Only show file info in trace mode
        .with_line_number(args.trace) // Only show line numbers in trace mode
        .with_span_events(if args.trace {
            FmtSpan::FULL
        } else {
            FmtSpan::NONE
        });

    // Configure output format
    let registry = tracing_subscriber::registry().with(filter);

    if args.json_logs {
        // JSON format
        let fmt_layer = fmt_layer.json();
        if args.timestamps {
            registry.with(fmt_layer).init();
        } else {
            registry.with(fmt_layer.without_time()).init();
        }
    } else {
        // Pretty format
        let fmt_layer = if args.no_color {
            fmt_layer.with_ansi(false)
        } else {
            fmt_layer.with_ansi(true)
        };

        if args.timestamps {
            registry.with(fmt_layer).init();
        } else {
            registry.with(fmt_layer.without_time()).init();
        }
    }

    // Log the selected configuration
    if args.trace {
        tracing::trace!("Tracing initialized with level: {}", log_level);
    } else if args.debug {
        tracing::debug!("Tracing initialized with level: {}", log_level);
    } else {
        tracing::info!("Tracing initialized with level: {}", log_level);
    }

    Ok(())
}

fn create_filter(level: &str) -> tracing_subscriber::EnvFilter {
    let base_filter = match level.to_lowercase().as_str() {
        "trace" => "lavalink_rust=trace,trace",
        "debug" => "lavalink_rust=debug,info",
        "info" => "lavalink_rust=info,warn",
        "warn" => "lavalink_rust=warn,error",
        "error" => "lavalink_rust=error",
        _ => {
            eprintln!(
                "Warning: Unknown log level '{}', defaulting to 'info'",
                level
            );
            "lavalink_rust=info,warn"
        }
    };

    tracing_subscriber::EnvFilter::new(base_filter)
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

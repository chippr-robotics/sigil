//! Sigil MCP Server
//!
//! A Model Context Protocol server that enables AI agents to sign blockchain
//! transactions using Sigil's MPC infrastructure.

use clap::{Parser, ValueEnum};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use sigil_mcp::McpServer;

/// Sigil MCP Server - MPC signing for AI agents
#[derive(Parser, Debug)]
#[command(name = "sigil-mcp")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Transport mechanism to use
    #[arg(short, long, value_enum, default_value = "stdio")]
    transport: Transport,

    /// Port for HTTP transport (ignored for stdio)
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Enable verbose logging (to stderr)
    #[arg(short, long)]
    verbose: bool,

    /// Use mock disk state (for testing without physical disk)
    #[arg(long)]
    mock: bool,

    /// Log level
    #[arg(long, value_enum, default_value = "info")]
    log_level: LogLevel,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Transport {
    /// Standard input/output (for Claude Desktop, VS Code, etc.)
    Stdio,
    /// HTTP with Server-Sent Events (for web clients) - coming soon
    Http,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    // Initialize logging to stderr (stdout is reserved for MCP protocol)
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::from(args.log_level))
        .with_writer(std::io::stderr)
        .with_ansi(false) // Disable ANSI colors for cleaner logs
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    info!("Sigil MCP Server v{} starting", env!("CARGO_PKG_VERSION"));

    // Create server with appropriate mode
    let server = if args.mock {
        info!("Using mock disk state");
        McpServer::with_mock()
    } else {
        info!("Connecting to Sigil daemon");
        match McpServer::with_daemon() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to connect to daemon: {}", e);
                eprintln!("Make sure the Sigil daemon is running: sigil-daemon start");
                eprintln!("Or use --mock flag for testing without daemon");
                std::process::exit(1);
            }
        }
    };

    // Run server with selected transport
    match args.transport {
        Transport::Stdio => {
            info!("Starting stdio transport");
            server.run_stdio().await?;
        }
        Transport::Http => {
            info!("HTTP transport not yet implemented");
            eprintln!("HTTP transport coming soon. Use --transport stdio for now.");
            std::process::exit(1);
        }
    }

    Ok(())
}

//! CLI command implementations

use clap::{Parser, Subcommand};

use crate::client::{ClientError, SigilClient};

/// Sigil CLI - MPC-secured blockchain signing
#[derive(Parser)]
#[command(name = "sigil")]
#[command(about = "MPC-secured floppy disk signing system")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to daemon socket
    #[arg(long, default_value = "/tmp/sigil.sock")]
    pub socket: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Check daemon status
    Status,

    /// Check signing disk status
    Disk,

    /// Sign a message hash
    Sign {
        /// Message hash to sign (hex encoded)
        #[arg(long)]
        message: String,

        /// Chain ID
        #[arg(long, default_value = "1")]
        chain_id: u32,

        /// Description for the usage log
        #[arg(long, default_value = "CLI signing")]
        description: String,
    },

    /// Update transaction hash after broadcast
    UpdateTx {
        /// Presig index
        #[arg(long)]
        presig_index: u32,

        /// Transaction hash (hex encoded)
        #[arg(long)]
        tx_hash: String,
    },

    /// Show presig count
    PresigCount,

    /// Import agent shard (agent's portion of master key)
    ImportAgentShard {
        /// Agent shard as hex string
        #[arg(long, group = "input")]
        hex: Option<String>,

        /// Path to file containing agent shard (hex encoded)
        #[arg(long, group = "input")]
        file: Option<std::path::PathBuf>,
    },

    /// Import child presignature shares
    ImportChildShares {
        /// Path to JSON file with child shares
        shares_file: std::path::PathBuf,

        /// Replace existing shares if child already imported
        #[arg(long)]
        replace: bool,
    },

    /// List imported children
    ListChildren,
}

/// Run the CLI
pub async fn run(cli: Cli) -> Result<(), ClientError> {
    let client = SigilClient::with_socket_path(cli.socket.into());

    match cli.command {
        Commands::Status => match client.ping().await {
            Ok(version) => {
                println!("Sigil daemon v{} is running", version);
            }
            Err(ClientError::DaemonNotRunning) => {
                println!("Sigil daemon is not running");
                println!("Start it with: sigil-daemon");
                return Err(ClientError::DaemonNotRunning);
            }
            Err(e) => return Err(e),
        },

        Commands::Disk => {
            let status = client.get_disk_status().await?;

            if status.detected {
                println!(
                    "Disk detected: sigil_{}",
                    status.child_id.unwrap_or_default()
                );
                println!(
                    "Presigs: {}/{} remaining",
                    status.presigs_remaining.unwrap_or(0),
                    status.presigs_total.unwrap_or(0)
                );
                println!("Expires: {} days", status.days_until_expiry.unwrap_or(0));
                println!(
                    "Valid: {}",
                    if status.is_valid.unwrap_or(false) {
                        "Yes"
                    } else {
                        "No"
                    }
                );
            } else {
                println!("No signing disk detected");
                println!("Please insert your Sigil floppy disk");
            }
        }

        Commands::Sign {
            message,
            chain_id,
            description,
        } => {
            // Check disk status first
            let status = client.get_disk_status().await?;
            if !status.detected {
                println!("No signing disk detected");
                println!("Please insert your Sigil floppy disk");
                return Err(ClientError::NoDiskDetected);
            }

            println!("Signing message...");
            let result = client.sign(&message, chain_id, &description).await?;

            println!("Signature: 0x{}", result.signature);
            println!("Presig index: {}", result.presig_index);
            println!("Proof hash: 0x{}", result.proof_hash);
        }

        Commands::UpdateTx {
            presig_index,
            tx_hash,
        } => {
            client.update_tx_hash(presig_index, &tx_hash).await?;
            println!("Transaction hash updated");
        }

        Commands::PresigCount => {
            let (remaining, total) = client.get_presig_count().await?;
            println!("Presigs: {}/{} remaining", remaining, total);
        }

        Commands::ImportAgentShard { hex, file } => {
            let hex_string = if let Some(hex_str) = hex {
                hex_str
            } else if let Some(file_path) = file {
                std::fs::read_to_string(&file_path)
                    .map_err(|e| ClientError::Io(e))?
                    .trim()
                    .to_string()
            } else {
                return Err(ClientError::RequestFailed(
                    "Must provide either --hex or --file".to_string(),
                ));
            };

            // Validate hex string
            let hex_str = hex_string.strip_prefix("0x").unwrap_or(&hex_string);
            if hex_str.len() != 64 {
                return Err(ClientError::RequestFailed(
                    "Agent shard must be 32 bytes (64 hex characters)".to_string(),
                ));
            }

            client.import_agent_shard(&hex_string).await?;
            println!("✓ Agent shard imported successfully");
            println!("The agent shard is now stored securely and ready for signing operations.");
        }

        Commands::ImportChildShares {
            shares_file,
            replace,
        } => {
            let shares_json = std::fs::read_to_string(&shares_file)
                .map_err(|e| ClientError::Io(e))?;

            client.import_child_shares(&shares_json, replace).await?;
            println!("✓ Child shares imported successfully");
        }

        Commands::ListChildren => {
            let children = client.list_children().await?;
            if children.is_empty() {
                println!("No children imported yet.");
                println!("Import child shares with: sigil import-child-shares <file>");
            } else {
                println!("Imported children:");
                for child_id in children {
                    println!("  - {}", child_id);
                }
            }
        }
    }

    Ok(())
}

//! Sigil CLI - Command-line interface for MPC signing with floppy disks
//!
//! This binary provides tools for agents to utilize keyshards stored on floppy disks
//! to manage blockchain transactions via Claude CLI integration.

use anyhow::Result;
use clap::{Parser, Subcommand};
use sha2::Digest;
use sigil_core::{
    blockchain::{Transaction, TransactionBuilder},
    disk::{DiskExpiry, DiskHeader, HEADER_SIZE, DEFAULT_PRESIG_COUNT},
    hd::{DerivationPath, MasterShard},
    mpc::ShardPair,
    presig::PresigColdShare,
};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sigil")]
#[command(about = "MPC-secured floppy disk signing system for blockchain transactions", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Disk management commands
    #[command(subcommand)]
    Disk(DiskCommands),

    /// Presignature management commands
    #[command(subcommand)]
    Presig(PresigCommands),

    /// Blockchain transaction commands
    #[command(subcommand)]
    Transaction(TransactionCommands),

    /// Storage commands
    #[command(subcommand)]
    Storage(StorageCommands),
}

#[derive(Subcommand)]
enum DiskCommands {
    /// Show disk information
    Info {
        /// Path to floppy disk mount
        #[arg(short, long)]
        path: PathBuf,
    },

    /// Create a new child disk
    Create {
        /// Cold master seed (hex)
        #[arg(long)]
        cold_seed: String,

        /// Agent master seed (hex)
        #[arg(long)]
        agent_seed: String,

        /// Child index
        #[arg(short, long)]
        child_index: u32,

        /// Output disk image path
        #[arg(short, long)]
        output: PathBuf,

        /// Number of presignatures to generate
        #[arg(short = 'n', long, default_value_t = DEFAULT_PRESIG_COUNT)]
        presig_count: u32,
    },

    /// Read disk header
    ReadHeader {
        /// Path to disk or disk image
        #[arg(short, long)]
        path: PathBuf,
    },

    /// Read usage log from disk
    ReadLog {
        /// Path to disk or disk image
        #[arg(short, long)]
        path: PathBuf,

        /// Show only last N entries
        #[arg(short = 'n', long)]
        last: Option<usize>,
    },
}

#[derive(Subcommand)]
enum PresigCommands {
    /// Generate presignatures for a child
    Generate {
        /// Cold master seed (hex)
        #[arg(long)]
        cold_seed: String,

        /// Agent master seed (hex)
        #[arg(long)]
        agent_seed: String,

        /// Child index
        #[arg(short, long)]
        child_index: u32,

        /// Number of presignatures
        #[arg(short = 'n', long, default_value_t = 1000)]
        count: usize,

        /// Output file for cold shares
        #[arg(long)]
        cold_output: PathBuf,

        /// Output file for agent shares
        #[arg(long)]
        agent_output: PathBuf,
    },

    /// Show presignature information
    Info {
        /// Presignature file path
        #[arg(short, long)]
        file: PathBuf,
    },
}

#[derive(Subcommand)]
enum TransactionCommands {
    /// Create a new transaction
    Create {
        /// Sender address
        #[arg(short, long)]
        from: String,

        /// Recipient address
        #[arg(short, long)]
        to: String,

        /// Amount to transfer
        #[arg(short, long)]
        amount: u64,

        /// Transaction nonce
        #[arg(short, long)]
        nonce: u64,

        /// Gas price
        #[arg(short = 'p', long, default_value = "1000000000")]
        gas_price: u64,

        /// Gas limit
        #[arg(short = 'l', long, default_value = "21000")]
        gas_limit: u64,

        /// Output JSON file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Display transaction details
    Show {
        /// Transaction JSON file
        #[arg(short, long)]
        file: PathBuf,
    },
}

#[derive(Subcommand)]
enum StorageCommands {
    /// Initialize storage directory
    Init {
        /// Storage path to initialize
        #[arg(short, long)]
        path: PathBuf,
    },

    /// Show storage information
    Info {
        /// Storage path
        #[arg(short, long)]
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Disk(cmd) => handle_disk_command(cmd).await,
        Commands::Presig(cmd) => handle_presig_command(cmd).await,
        Commands::Transaction(cmd) => handle_transaction_command(cmd).await,
        Commands::Storage(cmd) => handle_storage_command(cmd).await,
    }
}

async fn handle_disk_command(cmd: DiskCommands) -> Result<()> {
    match cmd {
        DiskCommands::Info { path } => {
            // Read header
            let data = fs::read(&path)?;
            if data.len() < HEADER_SIZE {
                anyhow::bail!("File too small to contain header");
            }

            let header = DiskHeader::from_bytes(&data[..HEADER_SIZE])?;

            println!("Disk Information:");
            println!("  Version: {}", header.version);
            println!("  Child ID: {}", hex::encode(&header.child_id));
            println!("  Child Pubkey: {}", hex::encode(&header.child_pubkey));
            println!("  Presignatures Total: {}", header.presig_total);
            println!("  Presignatures Used: {}", header.presig_used);
            println!("  Presignatures Remaining: {}", header.presig_total - header.presig_used);
            println!("  Created: {}", header.created_at);
            println!("\nExpiry Configuration:");
            println!("  Expires At: {}", header.expiry.expires_at);
            println!("  Days Until Expiry: {}", header.expiry.days_until_expiry());
            println!("  Reconciliation Deadline: {}", header.expiry.reconciliation_deadline);
            println!("  Uses Since Reconcile: {}/{}", 
                header.expiry.uses_since_reconcile, 
                header.expiry.max_uses_before_reconcile);
            
            if header.expiry.is_expired() {
                println!("\n⚠️  WARNING: Disk has EXPIRED!");
            } else if header.expiry.needs_reconciliation() {
                println!("\n⚠️  WARNING: Reconciliation REQUIRED!");
            } else if header.expiry.days_until_expiry() < 7 {
                println!("\n⚠️  WARNING: Disk expires in {} days!", header.expiry.days_until_expiry());
            }
        }

        DiskCommands::Create {
            cold_seed,
            agent_seed,
            child_index,
            output,
            presig_count,
        } => {
            println!("Creating child disk for index {}...", child_index);

            // Decode seeds
            let cold_seed_bytes = hex::decode(&cold_seed)?;
            let agent_seed_bytes = hex::decode(&agent_seed)?;

            // Create master shards
            let cold_master = MasterShard::from_seed(&cold_seed_bytes)?;
            let agent_master = MasterShard::from_seed(&agent_seed_bytes)?;

            // Create child shard pair
            let path = DerivationPath::bip44_ethereum(child_index);
            let pair = ShardPair::from_masters(&cold_master, &agent_master, &path)?;
            let pubkey = pair.pubkey()?;

            // Generate presignatures
            println!("Generating {} presignatures...", presig_count);
            let (cold_shares, _agent_shares) = pair.generate_presignatures(presig_count as usize)?;

            // Create child ID (hash of pubkey)
            let mut hasher = sha2::Sha256::new();
            sha2::Digest::update(&mut hasher, &pubkey);
            let hash = sha2::Digest::finalize(hasher);
            let mut child_id = [0u8; 32];
            child_id.copy_from_slice(&hash);

            // Create disk header
            let expiry = DiskExpiry::default_config();
            let header = DiskHeader::new(
                child_id,
                pubkey,
                path.to_bytes(),
                presig_count,
                expiry,
            )?;

            // Write disk image
            let mut disk_data = Vec::new();

            // Write header
            disk_data.extend_from_slice(&header.to_bytes());

            // Pad to presig table offset (0x0100 = 256)
            while disk_data.len() < 0x0100 {
                disk_data.push(0);
            }

            // Write presignature table
            for share in &cold_shares {
                disk_data.extend_from_slice(&share.to_disk_bytes());
            }

            // Pad remaining presig slots
            let presig_bytes_written = cold_shares.len() * PresigColdShare::DISK_SIZE;
            let presig_table_size = (presig_count as usize) * PresigColdShare::DISK_SIZE;
            for _ in 0..(presig_table_size - presig_bytes_written) {
                disk_data.push(0);
            }

            fs::write(&output, &disk_data)?;

            println!("✓ Disk created successfully!");
            println!("  Output: {}", output.display());
            println!("  Child ID: {}", hex::encode(&child_id));
            println!("  Pubkey: {}", hex::encode(&pubkey));
            println!("  Path: {}", path.to_string_path());
            println!("  Presignatures: {}", cold_shares.len());
            println!("  Size: {} bytes", disk_data.len());
        }

        DiskCommands::ReadHeader { path } => {
            let data = fs::read(&path)?;
            if data.len() < HEADER_SIZE {
                anyhow::bail!("File too small");
            }

            let header = DiskHeader::from_bytes(&data[..HEADER_SIZE])?;
            let json = serde_json::to_string_pretty(&header.expiry)?;
            
            println!("Header Details:");
            println!("  Magic: {}", String::from_utf8_lossy(&header.magic));
            println!("  Version: {}", header.version);
            println!("  Child ID: {}", hex::encode(&header.child_id));
            println!("  Child Pubkey: {}", hex::encode(&header.child_pubkey));
            println!("  Derivation Path: {}", hex::encode(&header.derivation_path));
            println!("  Presig Total/Used: {}/{}", header.presig_total, header.presig_used);
            println!("  Created: {}", header.created_at);
            println!("  Expiry Config: {}", json);
        }

        DiskCommands::ReadLog { path, last } => {
            println!("Reading usage log from {}...", path.display());
            println!("(Note: Usage log parsing not yet fully implemented)");
            
            if let Some(n) = last {
                println!("Showing last {} entries", n);
            }
        }
    }

    Ok(())
}

async fn handle_presig_command(cmd: PresigCommands) -> Result<()> {
    match cmd {
        PresigCommands::Generate {
            cold_seed,
            agent_seed,
            child_index,
            count,
            cold_output,
            agent_output,
        } => {
            println!("Generating {} presignatures for child {}...", count, child_index);

            let cold_seed_bytes = hex::decode(&cold_seed)?;
            let agent_seed_bytes = hex::decode(&agent_seed)?;

            let cold_master = MasterShard::from_seed(&cold_seed_bytes)?;
            let agent_master = MasterShard::from_seed(&agent_seed_bytes)?;

            let path = DerivationPath::bip44_ethereum(child_index);
            let pair = ShardPair::from_masters(&cold_master, &agent_master, &path)?;

            let (cold_shares, agent_shares) = pair.generate_presignatures(count)?;

            // Serialize and save
            let cold_json = serde_json::to_string_pretty(&cold_shares)?;
            let agent_json = serde_json::to_string_pretty(&agent_shares)?;

            fs::write(&cold_output, cold_json)?;
            fs::write(&agent_output, agent_json)?;

            println!("✓ Presignatures generated!");
            println!("  Cold shares: {}", cold_output.display());
            println!("  Agent shares: {}", agent_output.display());
        }

        PresigCommands::Info { file } => {
            let json = fs::read_to_string(&file)?;
            
            // Try to parse as cold shares
            if let Ok(cold_shares) = serde_json::from_str::<Vec<PresigColdShare>>(&json) {
                println!("Cold Presignature Shares:");
                println!("  Count: {}", cold_shares.len());
                println!("  Fresh: {}", cold_shares.iter().filter(|s| s.is_available()).count());
                println!("  Used: {}", cold_shares.iter().filter(|s| !s.is_available()).count());
            } else {
                anyhow::bail!("Could not parse presignature file");
            }
        }
    }

    Ok(())
}

async fn handle_transaction_command(cmd: TransactionCommands) -> Result<()> {
    match cmd {
        TransactionCommands::Create {
            from,
            to,
            amount,
            nonce,
            gas_price,
            gas_limit,
            output,
        } => {
            let tx = TransactionBuilder::new()
                .from(from)
                .to(to)
                .amount(amount)
                .nonce(nonce)
                .gas_price(gas_price)
                .gas_limit(gas_limit)
                .build()?;

            let json = tx.to_json()?;
            fs::write(&output, json)?;

            println!("✓ Transaction created!");
            println!("  ID: {}", tx.id);
            println!("  From: {}", tx.from);
            println!("  To: {}", tx.to);
            println!("  Amount: {}", tx.amount);
            println!("  Saved to: {}", output.display());
        }

        TransactionCommands::Show { file } => {
            let json = fs::read_to_string(&file)?;
            let tx = Transaction::from_json(&json)?;

            println!("Transaction Details:");
            println!("  ID: {}", tx.id);
            println!("  From: {}", tx.from);
            println!("  To: {}", tx.to);
            println!("  Amount: {}", tx.amount);
            println!("  Nonce: {}", tx.nonce);
            println!("  Gas Price: {}", tx.gas_price);
            println!("  Gas Limit: {}", tx.gas_limit);
            println!("  Timestamp: {}", tx.timestamp);
        }
    }

    Ok(())
}

async fn handle_storage_command(cmd: StorageCommands) -> Result<()> {
    match cmd {
        StorageCommands::Init { path } => {
            fs::create_dir_all(&path)?;
            println!("✓ Storage initialized at: {}", path.display());
        }

        StorageCommands::Info { path } => {
            if !path.exists() {
                anyhow::bail!("Path does not exist: {}", path.display());
            }

            println!("Storage Information:");
            println!("  Path: {}", path.display());
            println!("  Exists: {}", path.exists());
            println!("  Is Directory: {}", path.is_dir());
        }
    }

    Ok(())
}

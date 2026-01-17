//! Sigil CLI - Command-line interface for keyshard management
//!
//! This binary provides tools for agents to utilize keyshards stored on floppy disks
//! to manage blockchain transactions via Claude CLI integration.

use clap::{Parser, Subcommand};
use sigil::{
    blockchain::{Transaction, TransactionBuilder},
    crypto,
    keyshard::Keyshard,
    storage::StorageManager,
    Result, SigilError,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sigil")]
#[command(about = "A physical containment system for agentic MPC management", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Keyshard management commands
    #[command(subcommand)]
    Keyshard(KeyshardCommands),

    /// Blockchain transaction commands
    #[command(subcommand)]
    Transaction(TransactionCommands),

    /// Storage commands
    #[command(subcommand)]
    Storage(StorageCommands),
}

#[derive(Subcommand)]
enum KeyshardCommands {
    /// Create a new keyshard
    Create {
        /// Keyshard ID
        #[arg(short, long)]
        id: String,

        /// Shard index (1-based)
        #[arg(short = 'n', long)]
        index: u32,

        /// Total threshold required
        #[arg(short, long)]
        threshold: u32,

        /// Purpose description
        #[arg(short, long)]
        purpose: String,

        /// Associated blockchain address
        #[arg(short, long)]
        address: Option<String>,

        /// Key data (hex encoded)
        #[arg(short, long)]
        key_data: String,

        /// Storage path (e.g., /media/floppy)
        #[arg(short, long)]
        storage_path: PathBuf,
    },

    /// Read and display a keyshard
    Read {
        /// Keyshard ID
        #[arg(short, long)]
        id: String,

        /// Storage path
        #[arg(short, long)]
        storage_path: PathBuf,
    },

    /// List all keyshards in storage
    List {
        /// Storage path
        #[arg(short, long)]
        storage_path: PathBuf,
    },

    /// Delete a keyshard
    Delete {
        /// Keyshard ID
        #[arg(short, long)]
        id: String,

        /// Storage path
        #[arg(short, long)]
        storage_path: PathBuf,
    },

    /// Export keyshard to JSON
    Export {
        /// Keyshard ID
        #[arg(short, long)]
        id: String,

        /// Storage path
        #[arg(short, long)]
        storage_path: PathBuf,

        /// Output JSON file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Import keyshard from JSON
    Import {
        /// Input JSON file
        #[arg(short, long)]
        input: PathBuf,

        /// Storage path
        #[arg(short, long)]
        storage_path: PathBuf,
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

    /// Sign a transaction using keyshards
    Sign {
        /// Transaction JSON file
        #[arg(short, long)]
        transaction: PathBuf,

        /// Storage path with keyshards
        #[arg(short, long)]
        storage_path: PathBuf,

        /// Signer address
        #[arg(short = 'a', long)]
        signer: String,

        /// Output signed transaction file
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
    /// Show storage information
    Info {
        /// Storage path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// Initialize storage directory
    Init {
        /// Storage path to initialize
        #[arg(short, long)]
        path: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Keyshard(cmd) => handle_keyshard_command(cmd),
        Commands::Transaction(cmd) => handle_transaction_command(cmd),
        Commands::Storage(cmd) => handle_storage_command(cmd),
    }
}

fn handle_keyshard_command(cmd: KeyshardCommands) -> Result<()> {
    match cmd {
        KeyshardCommands::Create {
            id,
            index,
            threshold,
            purpose,
            address,
            key_data,
            storage_path,
        } => {
            // Decode hex key data
            let data = hex::decode(&key_data)
                .map_err(|e| SigilError::Keyshard(format!("Invalid hex data: {}", e)))?;

            // Create keyshard
            let shard = Keyshard::new(id, index, threshold, data, purpose, address)?;

            // Write to storage
            let manager = StorageManager::new(&storage_path)?;
            let path = manager.write_keyshard(&shard)?;

            println!("Keyshard created successfully!");
            println!("Saved to: {}", path.display());
            println!("ID: {}", shard.id);
            println!("Index: {}/{}", shard.index, shard.threshold);
        }

        KeyshardCommands::Read { id, storage_path } => {
            let manager = StorageManager::new(&storage_path)?;
            let shard = manager.read_keyshard(&id)?;

            println!("Keyshard Details:");
            println!("  ID: {}", shard.id);
            println!("  Index: {}/{}", shard.index, shard.threshold);
            println!("  Purpose: {}", shard.metadata.purpose);
            println!("  Created: {}", shard.metadata.created_at);
            println!("  Checksum: {}", shard.metadata.checksum);
            
            if let Some(addr) = &shard.metadata.blockchain_address {
                println!("  Blockchain Address: {}", addr);
            }

            println!("  Data Size: {} bytes", shard.data.len());
            println!("\nIntegrity Check: {}", 
                if shard.verify_integrity()? { "PASSED" } else { "FAILED" });
        }

        KeyshardCommands::List { storage_path } => {
            let manager = StorageManager::new(&storage_path)?;
            let ids = manager.list_keyshards()?;

            if ids.is_empty() {
                println!("No keyshards found in storage.");
            } else {
                println!("Found {} keyshard(s):", ids.len());
                for id in ids {
                    println!("  - {}", id);
                }
            }
        }

        KeyshardCommands::Delete { id, storage_path } => {
            let manager = StorageManager::new(&storage_path)?;
            manager.delete_keyshard(&id)?;
            println!("Keyshard '{}' deleted successfully.", id);
        }

        KeyshardCommands::Export {
            id,
            storage_path,
            output,
        } => {
            let manager = StorageManager::new(&storage_path)?;
            manager.export_keyshard_json(&id, &output)?;
            println!("Keyshard '{}' exported to: {}", id, output.display());
        }

        KeyshardCommands::Import {
            input,
            storage_path,
        } => {
            let manager = StorageManager::new(&storage_path)?;
            let path = manager.import_keyshard_json(&input)?;
            println!("Keyshard imported successfully to: {}", path.display());
        }
    }

    Ok(())
}

fn handle_transaction_command(cmd: TransactionCommands) -> Result<()> {
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
            std::fs::write(&output, json)?;

            println!("Transaction created successfully!");
            println!("Transaction ID: {}", tx.id);
            println!("Saved to: {}", output.display());
        }

        TransactionCommands::Sign {
            transaction,
            storage_path,
            signer,
            output,
        } => {
            // Read transaction
            let tx_json = std::fs::read_to_string(&transaction)?;
            let tx = Transaction::from_json(&tx_json)?;

            // Load keyshards from storage
            let manager = StorageManager::new(&storage_path)?;
            let shards = manager.load_all_keyshards()?;

            if shards.is_empty() {
                return Err(SigilError::Keyshard(
                    "No keyshards found in storage".to_string()
                ));
            }

            // Get signing message
            let message = tx.signing_message();

            // Simple signing demonstration using first keyshard
            // In production, this would use proper MPC signing protocol
            let key_data = &shards[0].data;
            let mut key = [0u8; 32];
            let copy_len = key_data.len().min(32);
            key[..copy_len].copy_from_slice(&key_data[..copy_len]);

            let signature = crypto::sign_data(&message, &key);

            // Create signed transaction
            let signed = sigil::blockchain::SignedTransaction::new(
                tx,
                signature,
                signer,
            );

            let json = signed.to_json()?;
            std::fs::write(&output, json)?;

            println!("Transaction signed successfully!");
            println!("Signature: {}", signed.signature);
            println!("Saved to: {}", output.display());
        }

        TransactionCommands::Show { file } => {
            let json = std::fs::read_to_string(&file)?;
            
            // Try to parse as signed transaction first
            if let Ok(signed) = sigil::blockchain::SignedTransaction::from_json(&json) {
                println!("Signed Transaction:");
                println!("  Signer: {}", signed.signer);
                println!("  Signature: {}", signed.signature);
                println!("\nTransaction Details:");
                print_transaction(&signed.transaction);
            } else {
                // Try regular transaction
                let tx = Transaction::from_json(&json)?;
                print_transaction(&tx);
            }
        }
    }

    Ok(())
}

fn print_transaction(tx: &Transaction) {
    println!("  ID: {}", tx.id);
    println!("  From: {}", tx.from);
    println!("  To: {}", tx.to);
    println!("  Amount: {}", tx.amount);
    println!("  Nonce: {}", tx.nonce);
    println!("  Gas Price: {}", tx.gas_price);
    println!("  Gas Limit: {}", tx.gas_limit);
    println!("  Timestamp: {}", tx.timestamp);
    if let Some(data) = &tx.data {
        println!("  Data: {} bytes", data.len());
    }
}

fn handle_storage_command(cmd: StorageCommands) -> Result<()> {
    match cmd {
        StorageCommands::Info { path } => {
            let manager = StorageManager::new(&path)?;
            let ids = manager.list_keyshards()?;
            let space = manager.get_available_space()?;

            println!("Storage Information:");
            println!("  Path: {}", path.display());
            println!("  Keyshards: {}", ids.len());
            println!("  Estimated Space: {} bytes ({:.2} MB)", 
                space, space as f64 / 1_048_576.0);
        }

        StorageCommands::Init { path } => {
            std::fs::create_dir_all(&path)?;
            println!("Storage initialized at: {}", path.display());
        }
    }

    Ok(())
}

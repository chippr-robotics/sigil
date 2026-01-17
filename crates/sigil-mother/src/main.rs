//! Sigil Mother - Air-gapped mother device CLI

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use sigil_mother::{
    ceremony::{CreateChildCeremony, ReconcileCeremony, RefillCeremony},
    keygen::MasterKeyGenerator,
    reconciliation,
    storage::{MasterShardData, MotherStorage},
    ChildRegistry,
};

/// Sigil Mother - Air-gapped MPC key management
#[derive(Parser)]
#[command(name = "sigil-mother")]
#[command(about = "Air-gapped mother device for Sigil MPC signing")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to mother device storage
    #[arg(long, default_value = "./sigil_mother_data")]
    data_dir: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the mother device with new master shards
    Init,

    /// Show mother device status
    Status,

    /// Create a new child disk
    CreateChild {
        /// Number of presignatures to generate
        #[arg(long, default_value = "1000")]
        presig_count: u32,

        /// Output path for disk image
        #[arg(long)]
        output: PathBuf,

        /// Output path for agent shares (JSON)
        #[arg(long)]
        agent_output: PathBuf,
    },

    /// List all registered children
    ListChildren,

    /// Reconcile a child disk
    Reconcile {
        /// Path to disk image
        #[arg(long)]
        disk: PathBuf,
    },

    /// Refill a child disk after reconciliation
    Refill {
        /// Path to disk image (will be modified)
        #[arg(long)]
        disk: PathBuf,

        /// Number of presignatures to generate
        #[arg(long, default_value = "1000")]
        presig_count: u32,

        /// Output path for new agent shares (JSON)
        #[arg(long)]
        agent_output: PathBuf,
    },

    /// Nullify a child (permanently disable)
    Nullify {
        /// Child ID (short form, e.g., "7a3f")
        #[arg(long)]
        child_id: String,

        /// Reason for nullification
        #[arg(long)]
        reason: String,
    },

    /// Export agent master shard (DANGEROUS - only for initial setup)
    ExportAgentShard {
        /// Output path for agent shard
        #[arg(long)]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sigil_mother=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();
    let storage = MotherStorage::new(cli.data_dir.clone())?;

    match cli.command {
        Commands::Init => {
            if storage.has_master_shard() {
                error!("Master shard already exists. Refusing to overwrite.");
                error!("If you want to reinitialize, manually delete the data directory.");
                return Ok(());
            }

            info!("Generating master shards...");
            let output = MasterKeyGenerator::generate()?;

            storage.save_master_shard(&output.cold_master_shard)?;

            println!("\n=== Master Key Generated ===\n");
            println!(
                "Master Public Key: 0x{}",
                hex::encode(output.master_pubkey.as_bytes())
            );
            println!("\n⚠️  IMPORTANT: The agent shard must be securely transferred to the agent device.");
            println!(
                "Agent Master Shard: 0x{}",
                hex::encode(&output.agent_master_shard)
            );
            println!(
                "\n⚠️  Write down or securely store the agent shard, then clear your terminal."
            );

            info!("Master shard saved to {:?}", cli.data_dir);
        }

        Commands::Status => {
            if !storage.has_master_shard() {
                println!("Mother device not initialized. Run 'sigil-mother init' first.");
                return Ok(());
            }

            let master = storage.load_master_shard()?;
            let registry = storage.load_registry()?;
            let (active, suspended, nullified) = registry.count_by_status();

            println!("\n=== Mother Device Status ===\n");
            println!(
                "Master Public Key: 0x{}",
                hex::encode(&master.master_pubkey)
            );
            println!(
                "Created: {}",
                chrono::DateTime::from_timestamp(master.created_at as i64, 0)
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
            );
            println!("Next Child Index: {}", master.next_child_index);
            println!("\nChildren:");
            println!("  Active:    {}", active);
            println!("  Suspended: {}", suspended);
            println!("  Nullified: {}", nullified);
        }

        Commands::CreateChild {
            presig_count,
            output,
            agent_output,
        } => {
            info!("Creating new child disk with {} presigs...", presig_count);

            let mut ceremony = CreateChildCeremony::new(storage);
            let result = ceremony.execute(presig_count)?;

            // Write disk image
            let disk_bytes = result.disk.to_bytes();
            std::fs::write(&output, &disk_bytes)?;
            info!("Disk image written to {:?}", output);

            // Write agent shares
            let agent_json = serde_json::to_string_pretty(&result.agent_shares)?;
            std::fs::write(&agent_output, &agent_json)?;
            info!("Agent shares written to {:?}", agent_output);

            println!("\n=== Child Created ===\n");
            println!("Child ID: {}", result.child_id.short());
            println!(
                "Public Key: 0x{}",
                hex::encode(result.child_pubkey.as_bytes())
            );
            println!(
                "Derivation Path: {}",
                result.derivation_path.to_string_path()
            );
            println!("Presigs: {}", presig_count);
            println!("\nDisk image: {:?}", output);
            println!("Agent shares: {:?}", agent_output);
            println!(
                "\n⚠️  Securely transfer agent shares to the agent device, then delete the file."
            );
        }

        Commands::ListChildren => {
            let registry = storage.load_registry()?;
            let children = registry.list_all();

            if children.is_empty() {
                println!("No children registered.");
                return Ok(());
            }

            println!("\n=== Registered Children ===\n");
            for child in children {
                let status = match &child.status {
                    sigil_core::ChildStatus::Active => "Active",
                    sigil_core::ChildStatus::Suspended => "Suspended",
                    sigil_core::ChildStatus::Nullified { .. } => "Nullified",
                };

                println!(
                    "{} | {} | {} | {} sigs | {} refills",
                    child.child_id.short(),
                    child.derivation_path.to_string_path(),
                    status,
                    child.total_signatures,
                    child.refill_count
                );
            }
        }

        Commands::Reconcile { disk } => {
            info!("Loading disk from {:?}...", disk);

            let disk_bytes = std::fs::read(&disk)?;
            let disk_format = sigil_core::DiskFormat::from_bytes(&disk_bytes)?;

            // Run analysis
            let analysis = reconciliation::analyze_disk(&disk_format);
            let report = reconciliation::generate_report(&analysis);
            println!("{}", report);

            // Run ceremony
            let mut ceremony = ReconcileCeremony::new(storage);
            let result = ceremony.execute(&disk_format)?;

            println!("\nRecommendation: {:?}", result.recommendation);
        }

        Commands::Refill {
            disk,
            presig_count,
            agent_output,
        } => {
            info!("Loading disk from {:?}...", disk);

            let disk_bytes = std::fs::read(&disk)?;
            let mut disk_format = sigil_core::DiskFormat::from_bytes(&disk_bytes)?;

            let mut ceremony = RefillCeremony::new(storage);
            let agent_shares = ceremony.execute(&mut disk_format, presig_count)?;

            // Write updated disk
            let updated_bytes = disk_format.to_bytes();
            std::fs::write(&disk, &updated_bytes)?;
            info!("Disk updated at {:?}", disk);

            // Write new agent shares
            let agent_json = serde_json::to_string_pretty(&agent_shares)?;
            std::fs::write(&agent_output, &agent_json)?;
            info!("New agent shares written to {:?}", agent_output);

            println!("\n=== Refill Complete ===\n");
            println!("New presigs: {}", presig_count);
            println!("Agent shares: {:?}", agent_output);
        }

        Commands::Nullify { child_id, reason } => {
            let mut registry = storage.load_registry()?;

            // Find child by short ID
            let full_id = registry
                .children
                .keys()
                .find(|k| k.starts_with(&child_id))
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Child not found: {}", child_id))?;

            let child_id_bytes = hex::decode(&full_id)?;
            let child_id = sigil_core::ChildId::new(
                child_id_bytes
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid child ID"))?,
            );

            registry.nullify_child(
                &child_id,
                sigil_core::NullificationReason::ManualRevocation,
                0,
            )?;

            storage.save_registry(&registry)?;

            warn!("Child {} has been NULLIFIED", child_id.short());
            warn!("Reason: {}", reason);
            println!("\n⚠️  The agent should also be notified to delete the corresponding shares.");
        }

        Commands::ExportAgentShard { output } => {
            warn!(
                "⚠️  DANGER: Exporting agent shard. This should only be done during initial setup."
            );
            warn!("⚠️  The agent shard gives the agent partial signing capability.");

            let master = storage.load_master_shard()?;

            // In a real implementation, we would derive and export the agent shard
            // For this demo, we show a warning
            println!("\nAgent shard export is not implemented in this demo.");
            println!("In production, the agent shard would be generated during 'init'");
            println!("and must be securely transferred to the agent device.");
        }
    }

    Ok(())
}

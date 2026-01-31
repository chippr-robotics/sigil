//! Sigil Daemon - Main entry point
//!
//! The daemon manages disk detection, agent shard storage, and signing operations.

use std::sync::Arc;
use std::thread;
use tokio::sync::RwLock;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use sigil_daemon::{AgentStore, DaemonConfig, DiskWatcher, IpcServer, MemoryManager, Signer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sigil_daemon=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Sigil daemon v{}", env!("CARGO_PKG_VERSION"));

    // Load or create config
    let config_path = std::env::var("SIGIL_CONFIG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            #[cfg(unix)]
            let fallback = std::path::PathBuf::from("/etc");
            #[cfg(windows)]
            let fallback = std::path::PathBuf::from(r"C:\ProgramData");

            dirs::config_dir()
                .unwrap_or(fallback)
                .join("sigil")
                .join("daemon.json")
        });

    let config = if config_path.exists() {
        DaemonConfig::load(&config_path)?
    } else {
        let config = DaemonConfig::default();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        config.save(&config_path)?;
        info!("Created default config at {:?}", config_path);
        config
    };

    // Ensure directories exist
    config.ensure_directories()?;

    // Initialize components
    let agent_store = Arc::new(RwLock::new(AgentStore::new(
        config.agent_store_path.clone(),
    )?));

    let disk_watcher = Arc::new(DiskWatcher::new(config.disk_mount_pattern.clone()));

    let signer = Arc::new(Signer::new(
        Arc::clone(&agent_store),
        Arc::clone(&disk_watcher),
        config.enable_zkvm_proving,
    ));

    let memory_manager = Arc::new(MemoryManager::new(Arc::clone(&disk_watcher)));

    let ipc_server = IpcServer::new(
        config.ipc_socket_path.clone(),
        Arc::clone(&disk_watcher),
        Arc::clone(&agent_store),
        Arc::clone(&signer),
        Arc::clone(&memory_manager),
    );

    // Start disk watcher in background on a dedicated thread
    // (udev types are not Send, so we need a separate runtime)
    let disk_watcher_handle = {
        let disk_watcher = Arc::clone(&disk_watcher);
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create disk watcher runtime");

            rt.block_on(async {
                if let Err(e) = disk_watcher.watch().await {
                    error!("Disk watcher error: {}", e);
                }
                let _ = tx.send(());
            });
        });

        rx
    };

    // Start IPC server
    let ipc_handle = tokio::spawn(async move {
        if let Err(e) = ipc_server.run().await {
            error!("IPC server error: {}", e);
        }
    });

    info!("Daemon started successfully");

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
        _ = disk_watcher_handle => {
            error!("Disk watcher exited unexpectedly");
        }
        _ = ipc_handle => {
            error!("IPC server exited unexpectedly");
        }
    }

    info!("Daemon shutting down");

    Ok(())
}

/// Helper module for dirs functionality
mod dirs {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        #[cfg(unix)]
        {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        }

        #[cfg(windows)]
        {
            std::env::var_os("APPDATA").map(PathBuf::from)
        }
    }
}

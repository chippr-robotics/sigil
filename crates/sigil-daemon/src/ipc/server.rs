//! IPC server implementation

use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use sigil_core::types::ChainId;

use crate::agent_store::AgentStore;
use crate::disk_watcher::DiskWatcher;
use crate::error::Result;
use crate::signer::{Signer, SigningRequest};

use super::connection::{IpcTransport, PlatformTransport};
use super::types::{parse_message_hash, parse_tx_hash, IpcRequest, IpcResponse};

/// IPC server
pub struct IpcServer {
    /// Socket path
    socket_path: PathBuf,

    /// Disk watcher
    disk_watcher: Arc<DiskWatcher>,

    /// Agent store
    agent_store: Arc<RwLock<AgentStore>>,

    /// Signer
    signer: Arc<Signer>,
}

impl IpcServer {
    /// Create a new IPC server
    pub fn new(
        socket_path: PathBuf,
        disk_watcher: Arc<DiskWatcher>,
        agent_store: Arc<RwLock<AgentStore>>,
        signer: Arc<Signer>,
    ) -> Self {
        Self {
            socket_path,
            disk_watcher,
            agent_store,
            signer,
        }
    }

    /// Start the IPC server
    pub async fn run(&self) -> Result<()> {
        let transport = PlatformTransport::bind(&self.socket_path).await?;

        info!("IPC server listening on {:?}", self.socket_path);

        loop {
            match transport.accept().await {
                Ok(stream) => {
                    let disk_watcher = Arc::clone(&self.disk_watcher);
                    let agent_store = Arc::clone(&self.agent_store);
                    let signer = Arc::clone(&self.signer);

                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_connection(stream, disk_watcher, agent_store, signer).await
                        {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }
}

/// Handle a single IPC connection
async fn handle_connection<S>(
    stream: S,
    disk_watcher: Arc<DiskWatcher>,
    agent_store: Arc<RwLock<AgentStore>>,
    signer: Arc<Signer>,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    while reader.read_line(&mut line).await? > 0 {
        let request: IpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let response = IpcResponse::Error {
                    message: format!("Invalid request: {}", e),
                };
                send_response(&mut writer, &response).await?;
                line.clear();
                continue;
            }
        };

        debug!("Received IPC request: {:?}", request);

        let response = handle_request(
            request,
            Arc::clone(&disk_watcher),
            Arc::clone(&agent_store),
            Arc::clone(&signer),
        )
        .await;

        send_response(&mut writer, &response).await?;
        line.clear();
    }

    Ok(())
}

/// Handle a single request
async fn handle_request(
    request: IpcRequest,
    disk_watcher: Arc<DiskWatcher>,
    agent_store: Arc<RwLock<AgentStore>>,
    signer: Arc<Signer>,
) -> IpcResponse {
    match request {
        IpcRequest::Ping => IpcResponse::Pong {
            version: env!("CARGO_PKG_VERSION").to_string(),
        },

        IpcRequest::GetDiskStatus => match disk_watcher.current_disk().await {
            Some(disk) => {
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let days_until_expiry = disk.header.expiry.days_until_expiry(current_time);
                let is_valid = disk.header.validate(current_time).is_ok();

                IpcResponse::DiskStatus {
                    detected: true,
                    child_id: Some(disk.header.child_id.short()),
                    presigs_remaining: Some(disk.header.presigs_remaining()),
                    presigs_total: Some(disk.header.presig_total),
                    days_until_expiry: Some(days_until_expiry),
                    is_valid: Some(is_valid),
                }
            }
            None => IpcResponse::DiskStatus {
                detected: false,
                child_id: None,
                presigs_remaining: None,
                presigs_total: None,
                days_until_expiry: None,
                is_valid: None,
            },
        },

        IpcRequest::Sign {
            message_hash,
            chain_id,
            description,
        } => {
            // Parse message hash
            let message_hash = match parse_message_hash(&message_hash) {
                Ok(h) => h,
                Err(e) => {
                    return IpcResponse::Error {
                        message: format!("Invalid message hash: {}", e),
                    }
                }
            };

            let request = SigningRequest {
                message_hash,
                chain_id: ChainId::new(chain_id),
                description,
            };

            match signer.sign(request).await {
                Ok(result) => IpcResponse::SignResult {
                    signature: hex::encode(result.signature.as_bytes()),
                    presig_index: result.presig_index,
                    proof_hash: hex::encode(result.proof_hash.as_bytes()),
                },
                Err(e) => IpcResponse::Error {
                    message: format!("Signing failed: {}", e),
                },
            }
        }

        IpcRequest::UpdateTxHash {
            presig_index,
            tx_hash,
        } => {
            let tx_hash = match parse_tx_hash(&tx_hash) {
                Ok(h) => h,
                Err(e) => {
                    return IpcResponse::Error {
                        message: format!("Invalid tx hash: {}", e),
                    }
                }
            };

            match signer.update_tx_hash(presig_index, tx_hash).await {
                Ok(()) => IpcResponse::Ok,
                Err(e) => IpcResponse::Error {
                    message: format!("Failed to update tx hash: {}", e),
                },
            }
        }

        IpcRequest::ListChildren => {
            let store = agent_store.read().await;
            match store.list_children() {
                Ok(children) => IpcResponse::Children {
                    child_ids: children.iter().map(|c| c.short()).collect(),
                },
                Err(e) => IpcResponse::Error {
                    message: format!("Failed to list children: {}", e),
                },
            }
        }

        IpcRequest::GetPresigCount => match disk_watcher.current_disk().await {
            Some(disk) => IpcResponse::PresigCount {
                remaining: disk.header.presigs_remaining(),
                total: disk.header.presig_total,
            },
            None => IpcResponse::Error {
                message: "No disk detected".to_string(),
            },
        },

        IpcRequest::ImportAgentShard { agent_shard_hex } => {
            // Parse hex string
            let agent_shard_hex = agent_shard_hex
                .strip_prefix("0x")
                .unwrap_or(&agent_shard_hex);
            let mut shard = [0u8; 32];
            match hex::decode_to_slice(agent_shard_hex, &mut shard) {
                Ok(()) => {
                    let mut store = agent_store.write().await;
                    match store.import_agent_master_shard(shard) {
                        Ok(()) => IpcResponse::Ok,
                        Err(e) => IpcResponse::Error {
                            message: format!("Failed to import agent shard: {}", e),
                        },
                    }
                }
                Err(e) => IpcResponse::Error {
                    message: format!("Invalid hex string: {}", e),
                },
            }
        }

        IpcRequest::ImportChildShares {
            shares_json,
            replace,
        } => {
            // Parse JSON
            match serde_json::from_str::<crate::agent_store::AgentChildData>(&shares_json) {
                Ok(child_data) => {
                    let mut store = agent_store.write().await;

                    // Check if child already exists
                    let child_id = child_data.child_id;
                    let exists = store.load_child(&child_id).is_ok();

                    if exists && !replace {
                        IpcResponse::Error {
                            message: format!(
                                "Child {} already exists. Use --replace to overwrite.",
                                child_id.short()
                            ),
                        }
                    } else {
                        match store.store_child(child_data) {
                            Ok(()) => IpcResponse::Ok,
                            Err(e) => IpcResponse::Error {
                                message: format!("Failed to import child shares: {}", e),
                            },
                        }
                    }
                }
                Err(e) => IpcResponse::Error {
                    message: format!("Invalid JSON: {}", e),
                },
            }
        }
    }
}

/// Send a response over the socket
async fn send_response<W>(writer: &mut W, response: &IpcResponse) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let json = serde_json::to_string(response)?;
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}

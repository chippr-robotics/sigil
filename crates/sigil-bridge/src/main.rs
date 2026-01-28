//! sigil-bridge: HTTP bridge server for sigil-daemon
//!
//! This server provides an HTTP API that proxies requests to the sigil-daemon's
//! IPC interface, enabling mobile apps and other HTTP clients to communicate
//! with the daemon.

use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

mod client;

use client::DaemonClient;

#[derive(Parser, Debug)]
#[command(name = "sigil-bridge")]
#[command(about = "HTTP bridge server for sigil-daemon")]
struct Args {
    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Port to bind to
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Path to sigil-daemon IPC socket
    #[arg(long, default_value = "/tmp/sigil.sock")]
    socket_path: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Clone)]
struct AppState {
    daemon_client: Arc<DaemonClient>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let filter = if args.verbose {
        "sigil_bridge=debug,tower_http=debug"
    } else {
        "sigil_bridge=info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    info!("Starting sigil-bridge HTTP server");
    info!("Daemon socket: {}", args.socket_path);

    let daemon_client = Arc::new(DaemonClient::new(&args.socket_path));
    let state = AppState { daemon_client };

    // Configure CORS for mobile app access
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Health check
        .route("/health", get(health))
        // API routes
        .route("/api/ping", post(ping))
        .route("/api/disk-status", post(get_disk_status))
        .route("/api/presig-count", post(get_presig_count))
        .route("/api/sign", post(sign))
        .route("/api/sign-frost", post(sign_frost))
        .route("/api/address", post(get_address))
        .route("/api/update-tx-hash", post(update_tx_hash))
        .route("/api/list-children", post(list_children))
        .route("/api/import-agent-shard", post(import_agent_shard))
        .route("/api/import-child-shares", post(import_child_shares))
        .route("/api/schemes", get(list_schemes))
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Health check endpoint
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

// Ping daemon
async fn ping(State(state): State<AppState>) -> impl IntoResponse {
    match state.daemon_client.ping().await {
        Ok(version) => (StatusCode::OK, Json(serde_json::json!({
            "type": "Pong",
            "version": version
        }))),
        Err(e) => {
            warn!("Ping failed: {}", e);
            (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({
                "type": "Error",
                "message": e.to_string()
            })))
        }
    }
}

// Get disk status
async fn get_disk_status(State(state): State<AppState>) -> impl IntoResponse {
    match state.daemon_client.get_disk_status().await {
        Ok(status) => (StatusCode::OK, Json(status)),
        Err(e) => {
            warn!("Get disk status failed: {}", e);
            (StatusCode::OK, Json(serde_json::json!({
                "detected": false,
                "error": e.to_string()
            })))
        }
    }
}

// Get presig count
async fn get_presig_count(State(state): State<AppState>) -> impl IntoResponse {
    match state.daemon_client.get_presig_count().await {
        Ok(count) => (StatusCode::OK, Json(count)),
        Err(e) => {
            warn!("Get presig count failed: {}", e);
            (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({
                "type": "Error",
                "message": e.to_string()
            })))
        }
    }
}

#[derive(Debug, Deserialize)]
struct SignRequest {
    message_hash: String,
    chain_id: u32,
    description: String,
}

// Sign EVM transaction
async fn sign(
    State(state): State<AppState>,
    Json(req): Json<SignRequest>,
) -> impl IntoResponse {
    match state.daemon_client.sign(&req.message_hash, req.chain_id, &req.description).await {
        Ok(result) => (StatusCode::OK, Json(result)),
        Err(e) => {
            warn!("Sign failed: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "type": "Error",
                "message": e.to_string()
            })))
        }
    }
}

#[derive(Debug, Deserialize)]
struct SignFrostRequest {
    scheme: String,
    message_hash: String,
    description: String,
}

// Sign with FROST
async fn sign_frost(
    State(state): State<AppState>,
    Json(req): Json<SignFrostRequest>,
) -> impl IntoResponse {
    match state.daemon_client.sign_frost(&req.scheme, &req.message_hash, &req.description).await {
        Ok(result) => (StatusCode::OK, Json(result)),
        Err(e) => {
            warn!("Sign FROST failed: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "type": "Error",
                "message": e.to_string()
            })))
        }
    }
}

#[derive(Debug, Deserialize)]
struct GetAddressRequest {
    scheme: Option<String>,
    format: String,
    cosmos_prefix: Option<String>,
}

// Get address
async fn get_address(
    State(state): State<AppState>,
    Json(req): Json<GetAddressRequest>,
) -> impl IntoResponse {
    match state.daemon_client.get_address(req.scheme.as_deref(), &req.format, req.cosmos_prefix.as_deref()).await {
        Ok(address) => (StatusCode::OK, Json(address)),
        Err(e) => {
            warn!("Get address failed: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "type": "Error",
                "message": e.to_string()
            })))
        }
    }
}

#[derive(Debug, Deserialize)]
struct UpdateTxHashRequest {
    presig_index: u32,
    tx_hash: String,
}

// Update transaction hash
async fn update_tx_hash(
    State(state): State<AppState>,
    Json(req): Json<UpdateTxHashRequest>,
) -> impl IntoResponse {
    match state.daemon_client.update_tx_hash(req.presig_index, &req.tx_hash).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({
            "type": "Ok"
        }))),
        Err(e) => {
            warn!("Update tx hash failed: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "type": "Error",
                "message": e.to_string()
            })))
        }
    }
}

// List children
async fn list_children(State(state): State<AppState>) -> impl IntoResponse {
    match state.daemon_client.list_children().await {
        Ok(children) => (StatusCode::OK, Json(serde_json::json!({
            "type": "Children",
            "child_ids": children
        }))),
        Err(e) => {
            warn!("List children failed: {}", e);
            (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({
                "type": "Error",
                "message": e.to_string()
            })))
        }
    }
}

#[derive(Debug, Deserialize)]
struct ImportAgentShardRequest {
    agent_shard_hex: String,
}

// Import agent shard
async fn import_agent_shard(
    State(state): State<AppState>,
    Json(req): Json<ImportAgentShardRequest>,
) -> impl IntoResponse {
    match state.daemon_client.import_agent_shard(&req.agent_shard_hex).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({
            "type": "Ok"
        }))),
        Err(e) => {
            warn!("Import agent shard failed: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "type": "Error",
                "message": e.to_string()
            })))
        }
    }
}

#[derive(Debug, Deserialize)]
struct ImportChildSharesRequest {
    shares_json: String,
    replace: bool,
}

// Import child shares
async fn import_child_shares(
    State(state): State<AppState>,
    Json(req): Json<ImportChildSharesRequest>,
) -> impl IntoResponse {
    match state.daemon_client.import_child_shares(&req.shares_json, req.replace).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({
            "type": "Ok"
        }))),
        Err(e) => {
            warn!("Import child shares failed: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "type": "Error",
                "message": e.to_string()
            })))
        }
    }
}

// List supported schemes
async fn list_schemes() -> impl IntoResponse {
    Json(serde_json::json!({
        "schemes": [
            {
                "name": "ecdsa",
                "description": "ECDSA on secp256k1 - Ethereum and EVM-compatible chains",
                "chains": ["Ethereum", "Polygon", "Arbitrum", "Optimism", "Base", "BSC", "Avalanche"]
            },
            {
                "name": "taproot",
                "description": "BIP-340 Schnorr signatures - Bitcoin Taproot",
                "chains": ["Bitcoin (Taproot)"]
            },
            {
                "name": "ed25519",
                "description": "Ed25519 signatures - Solana, Cosmos, and others",
                "chains": ["Solana", "Cosmos", "Near", "Polkadot", "Cardano"]
            },
            {
                "name": "ristretto255",
                "description": "Ristretto255 signatures - Zcash shielded transactions",
                "chains": ["Zcash (shielded)"]
            }
        ]
    }))
}

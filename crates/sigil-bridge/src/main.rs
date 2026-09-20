//! sigil-bridge: HTTP bridge server for sigil-daemon
//!
//! # NOT IN THE TRUSTED COMPUTING BASE
//!
//! This crate terminates HTTP and proxies to the daemon's IPC socket so that
//! convenience clients (the mobile app) can reach it. It is **out of TCB** as
//! defined in `.specify/memory/constitution.md`:
//!
//! - It is excluded from the workspace's `default-members`; a bare
//!   `cargo build` at the repository root does not produce this binary.
//! - It is `publish = false`.
//! - It binds loopback only unless the operator explicitly acknowledges
//!   otherwise, and it authenticates every `/api/*` caller with a bearer token.
//! - It transports **no key material**. Shard import and export cross the air
//!   gap by physical media and in-TCB tooling, never over this server.
//!
//! None of that makes the bridge trusted. The security boundary remains the
//! physically inserted disk: the daemon re-reads presignature shares from the
//! block device on every signing operation and fails closed without one. The
//! bridge cannot manufacture consent; the worst an authenticated caller can do
//! is spend presignatures the operator physically inserted.

use anyhow::{bail, Context, Result};
use axum::{
    extract::State,
    http::{HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use serde::Deserialize;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

mod auth;
mod client;

use auth::BridgeToken;
use client::DaemonClient;

/// Acknowledgement required before the bridge will leave loopback.
const NON_LOOPBACK_ACK: &str = "--allow-non-loopback";

#[derive(Parser, Debug)]
#[command(name = "sigil-bridge")]
#[command(
    about = "HTTP bridge server for sigil-daemon (NOT part of the Sigil TCB)",
    long_about = "HTTP bridge server for sigil-daemon.\n\n\
                  This binary is NOT part of the Sigil trusted computing base. It binds \
                  loopback only by default and requires a bearer token on every /api route. \
                  It never transports shard or key material. Signing still requires a \
                  physically inserted disk; the bridge cannot produce a signature on its own."
)]
struct Args {
    /// Host to bind to. Non-loopback addresses require `--allow-non-loopback`.
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to bind to
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Path to sigil-daemon IPC socket
    #[arg(long, default_value = "/run/sigil/sigil.sock")]
    socket_path: String,

    /// Acknowledge that binding a non-loopback address exposes a signing
    /// endpoint to the network. Required for any host other than loopback.
    #[arg(long)]
    allow_non_loopback: bool,

    /// File containing the bearer token clients must present. Falls back to
    /// $SIGIL_BRIDGE_TOKEN, then to a freshly generated token persisted with
    /// owner-only permissions.
    #[arg(long)]
    token_file: Option<PathBuf>,

    /// Explicitly permit a browser origin (repeatable). Omitted by default:
    /// no cross-origin access is granted unless an operator asks for it.
    #[arg(long = "allow-origin")]
    allow_origins: Vec<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Clone)]
struct AppState {
    daemon_client: Arc<DaemonClient>,
}

/// Decide whether the requested bind address is permitted.
///
/// Loopback is always allowed. Anything else requires explicit operator
/// acknowledgement, because it turns a local convenience shim into a
/// network-reachable path to an inserted disk.
fn check_bind_policy(host: &str, acknowledged: bool) -> Result<()> {
    let ip: IpAddr = host
        .parse()
        .with_context(|| format!("`--host {host}` is not a valid IP address"))?;

    if ip.is_loopback() || acknowledged {
        return Ok(());
    }

    bail!(
        "refusing to bind {host}: this exposes signing endpoints beyond this machine.\n\
         Loopback ({}) is the supported configuration; reach the bridge from another \n\
         device over an SSH tunnel or VPN rather than binding a LAN interface.\n\
         If you understand the exposure and still want it, pass {NON_LOOPBACK_ACK}.",
        if ip.is_ipv6() { "::1" } else { "127.0.0.1" }
    )
}

/// Build the CORS layer from explicitly allowed origins.
///
/// Returns `None` when the operator named no origins, in which case no CORS
/// headers are emitted at all and browsers refuse cross-origin responses. The
/// previous wildcard policy made every browser on the network a confused
/// deputy for `/api/sign`.
fn build_cors_layer(origins: &[String]) -> Result<Option<CorsLayer>> {
    if origins.is_empty() {
        return Ok(None);
    }

    let parsed = origins
        .iter()
        .map(|origin| {
            HeaderValue::from_str(origin)
                .with_context(|| format!("`--allow-origin {origin}` is not a valid origin"))
        })
        .collect::<Result<Vec<_>>>()?;

    warn!(
        "Cross-origin access explicitly granted to: {}",
        origins.join(", ")
    );

    Ok(Some(
        CorsLayer::new()
            .allow_origin(parsed)
            .allow_methods([Method::GET, Method::POST])
            .allow_headers([
                axum::http::header::AUTHORIZATION,
                axum::http::header::CONTENT_TYPE,
            ]),
    ))
}

/// Assemble the router.
///
/// `/health` is unauthenticated and discloses nothing but liveness. Every
/// `/api` route sits behind the bearer-token middleware, so an unauthenticated
/// request is rejected before any IPC call to the daemon is made.
fn build_router(state: AppState, token: BridgeToken, cors: Option<CorsLayer>) -> Router {
    let api = Router::new()
        .route("/ping", post(ping))
        .route("/disk-status", post(get_disk_status))
        .route("/presig-count", post(get_presig_count))
        .route("/sign", post(sign))
        .route("/sign-frost", post(sign_frost))
        .route("/address", post(get_address))
        .route("/update-tx-hash", post(update_tx_hash))
        .route("/list-children", post(list_children))
        .route("/schemes", get(list_schemes))
        .route_layer(axum::middleware::from_fn_with_state(
            token,
            auth::require_token,
        ))
        .with_state(state);

    let app = Router::new()
        .route("/health", get(health))
        .nest("/api", api);

    let app = match cors {
        Some(layer) => app.layer(layer),
        None => app,
    };

    app.layer(TraceLayer::new_for_http())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let filter = if args.verbose {
        "sigil_bridge=debug,tower_http=debug"
    } else {
        "sigil_bridge=info"
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!("Starting sigil-bridge HTTP server");
    warn!(
        "sigil-bridge is NOT part of the Sigil TCB. Signing still requires a \
         physically inserted disk; this server cannot create one."
    );

    check_bind_policy(&args.host, args.allow_non_loopback)?;
    if args.allow_non_loopback {
        warn!(
            "Binding {} beyond loopback by explicit acknowledgement. Any host that \
             can route here and holds the bearer token can spend presignatures from \
             an inserted disk.",
            args.host
        );
    }

    let token = BridgeToken::resolve(args.token_file.as_deref(), None)?;
    let cors = build_cors_layer(&args.allow_origins)?;

    info!("Daemon socket: {}", args.socket_path);
    let daemon_client = Arc::new(DaemonClient::new(&args.socket_path));
    let state = AppState { daemon_client };

    let app = build_router(state, token, cors);

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Liveness only. Deliberately discloses no disk state, presignature count,
/// address, or child identifier: an unauthenticated caller learns nothing
/// about what is inserted.
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

// Ping daemon
async fn ping(State(state): State<AppState>) -> impl IntoResponse {
    match state.daemon_client.ping().await {
        Ok(version) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "type": "Pong",
                "version": version
            })),
        ),
        Err(e) => {
            warn!("Ping failed: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "type": "Error",
                    "message": e.to_string()
                })),
            )
        }
    }
}

// Get disk status
async fn get_disk_status(State(state): State<AppState>) -> impl IntoResponse {
    match state.daemon_client.get_disk_status().await {
        Ok(status) => (StatusCode::OK, Json(status)),
        Err(e) => {
            warn!("Get disk status failed: {}", e);
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "detected": false,
                    "error": e.to_string()
                })),
            )
        }
    }
}

// Get presig count
async fn get_presig_count(State(state): State<AppState>) -> impl IntoResponse {
    match state.daemon_client.get_presig_count().await {
        Ok(count) => (StatusCode::OK, Json(count)),
        Err(e) => {
            warn!("Get presig count failed: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "type": "Error",
                    "message": e.to_string()
                })),
            )
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
async fn sign(State(state): State<AppState>, Json(req): Json<SignRequest>) -> impl IntoResponse {
    match state
        .daemon_client
        .sign(&req.message_hash, req.chain_id, &req.description)
        .await
    {
        Ok(result) => (StatusCode::OK, Json(result)),
        Err(e) => {
            warn!("Sign failed: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "type": "Error",
                    "message": e.to_string()
                })),
            )
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
    match state
        .daemon_client
        .sign_frost(&req.scheme, &req.message_hash, &req.description)
        .await
    {
        Ok(result) => (StatusCode::OK, Json(result)),
        Err(e) => {
            warn!("Sign FROST failed: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "type": "Error",
                    "message": e.to_string()
                })),
            )
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
    match state
        .daemon_client
        .get_address(
            req.scheme.as_deref(),
            &req.format,
            req.cosmos_prefix.as_deref(),
        )
        .await
    {
        Ok(address) => (StatusCode::OK, Json(address)),
        Err(e) => {
            warn!("Get address failed: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "type": "Error",
                    "message": e.to_string()
                })),
            )
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
    match state
        .daemon_client
        .update_tx_hash(req.presig_index, &req.tx_hash)
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "type": "Ok"
            })),
        ),
        Err(e) => {
            warn!("Update tx hash failed: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "type": "Error",
                    "message": e.to_string()
                })),
            )
        }
    }
}

// List children
async fn list_children(State(state): State<AppState>) -> impl IntoResponse {
    match state.daemon_client.list_children().await {
        Ok(children) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "type": "Children",
                "child_ids": children
            })),
        ),
        Err(e) => {
            warn!("List children failed: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "type": "Error",
                    "message": e.to_string()
                })),
            )
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use clap::CommandFactory;
    use tower::ServiceExt;

    const TEST_TOKEN: &str = "0123456789abcdef-test-token";

    fn test_router() -> Router {
        // The socket path is deliberately bogus: every test here asserts on
        // behaviour that must happen *before* the daemon is ever contacted.
        let state = AppState {
            daemon_client: Arc::new(DaemonClient::new("/nonexistent/sigil-test.sock")),
        };
        build_router(state, BridgeToken::from_value(TEST_TOKEN), None)
    }

    async fn request(method: Method, uri: &str, token: Option<&str>) -> axum::http::Response<Body> {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        let body = Body::from(r#"{"message_hash":"0x00","chain_id":1,"description":"test"}"#);
        test_router()
            .oneshot(builder.body(body).unwrap())
            .await
            .unwrap()
    }

    #[test]
    fn default_host_is_loopback() {
        let args = Args::parse_from(["sigil-bridge"]);
        assert_eq!(args.host, "127.0.0.1");
        assert!(!args.allow_non_loopback);
        check_bind_policy(&args.host, args.allow_non_loopback)
            .expect("default configuration must be permitted");
    }

    #[test]
    fn default_socket_path_is_not_world_writable() {
        let args = Args::parse_from(["sigil-bridge"]);
        assert!(
            !args.socket_path.starts_with("/tmp"),
            "default IPC socket must not live in a world-writable directory"
        );
    }

    #[test]
    fn non_loopback_bind_is_refused_without_acknowledgement() {
        let err = check_bind_policy("0.0.0.0", false).unwrap_err().to_string();
        assert!(err.contains("refusing to bind"));
        assert!(err.contains(NON_LOOPBACK_ACK));

        assert!(check_bind_policy("192.168.1.10", false).is_err());
        assert!(check_bind_policy("::", false).is_err());
    }

    #[test]
    fn loopback_variants_are_permitted() {
        assert!(check_bind_policy("127.0.0.1", false).is_ok());
        assert!(check_bind_policy("127.0.0.53", false).is_ok());
        assert!(check_bind_policy("::1", false).is_ok());
    }

    #[test]
    fn acknowledged_non_loopback_bind_is_permitted() {
        assert!(check_bind_policy("0.0.0.0", true).is_ok());
    }

    #[test]
    fn invalid_host_is_rejected() {
        assert!(check_bind_policy("not-an-ip", false).is_err());
        assert!(check_bind_policy("not-an-ip", true).is_err());
    }

    #[test]
    fn no_allowed_origins_means_no_cors_layer() {
        assert!(build_cors_layer(&[]).unwrap().is_none());
    }

    #[test]
    fn explicit_origins_build_a_layer() {
        let origins = vec!["https://app.example".to_string()];
        assert!(build_cors_layer(&origins).unwrap().is_some());
    }

    #[test]
    fn invalid_origin_is_rejected() {
        let origins = vec!["not a header\nvalue".to_string()];
        assert!(build_cors_layer(&origins).is_err());
    }

    #[tokio::test]
    async fn sign_without_token_is_unauthorized() {
        let response = request(Method::POST, "/api/sign", None).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn sign_with_wrong_token_is_unauthorized() {
        let response = request(Method::POST, "/api/sign", Some("wrong-token-wrong!!")).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn sign_frost_without_token_is_unauthorized() {
        let response = request(Method::POST, "/api/sign-frost", None).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn every_api_route_requires_a_token() {
        for (method, uri) in [
            (Method::POST, "/api/ping"),
            (Method::POST, "/api/disk-status"),
            (Method::POST, "/api/presig-count"),
            (Method::POST, "/api/sign"),
            (Method::POST, "/api/sign-frost"),
            (Method::POST, "/api/address"),
            (Method::POST, "/api/update-tx-hash"),
            (Method::POST, "/api/list-children"),
            (Method::GET, "/api/schemes"),
        ] {
            let response = request(method, uri, None).await;
            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "{uri} must require a bearer token"
            );
        }
    }

    #[tokio::test]
    async fn key_material_routes_do_not_exist() {
        for uri in ["/api/import-agent-shard", "/api/import-child-shares"] {
            // Authenticated, so a 404 proves the route is gone rather than guarded.
            let response = request(Method::POST, uri, Some(TEST_TOKEN)).await;
            assert_eq!(
                response.status(),
                StatusCode::NOT_FOUND,
                "{uri} must not exist on the bridge"
            );
        }
    }

    #[tokio::test]
    async fn health_is_open_and_discloses_nothing() {
        let response = request(Method::GET, "/health", None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json, serde_json::json!({ "status": "ok" }));

        let object = json.as_object().unwrap();
        for leaky in [
            "detected",
            "presigs_remaining",
            "presigs_total",
            "child_id",
            "address",
            "public_key",
        ] {
            assert!(
                !object.contains_key(leaky),
                "/health must not expose {leaky}"
            );
        }
    }

    #[test]
    fn cli_exposes_no_key_import_flags() {
        for arg in Args::command().get_arguments() {
            let id = arg.get_id().as_str();
            assert!(
                !id.contains("shard") && !id.contains("import"),
                "CLI must not offer shard transport, found `--{id}`"
            );
        }
    }

    #[test]
    fn cli_help_states_it_is_outside_the_tcb() {
        let rendered = Args::command().render_long_help().to_string();
        assert!(
            rendered.contains("NOT part of the Sigil trusted computing base"),
            "operators must be told this binary is out of TCB"
        );
    }
}

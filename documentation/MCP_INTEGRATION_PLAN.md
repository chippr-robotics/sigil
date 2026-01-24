# Sigil MCP Integration Plan

## Executive Summary

This document outlines the comprehensive plan to integrate the Model Context Protocol (MCP) into Sigil, enabling any MCP-compatible AI agent (Claude, GPT-4, Gemini, open-source agents) to securely sign blockchain transactions using Sigil's MPC infrastructure.

**Goal**: Transform Sigil from a Claude-specific tool into a universal agent-compatible MPC signing server.

---

## Table of Contents

1. [MCP Protocol Overview](#1-mcp-protocol-overview)
2. [Current Sigil Architecture](#2-current-sigil-architecture)
3. [Integration Architecture](#3-integration-architecture)
4. [New Crate: sigil-mcp](#4-new-crate-sigil-mcp)
5. [Tool Definitions](#5-tool-definitions)
6. [Resource Definitions](#6-resource-definitions)
7. [Prompt Definitions](#7-prompt-definitions)
8. [Transport Implementation](#8-transport-implementation)
9. [Security Considerations](#9-security-considerations)
10. [Implementation Phases](#10-implementation-phases)
11. [Testing Strategy](#11-testing-strategy)
12. [Migration Path](#12-migration-path)

---

## 1. MCP Protocol Overview

### What is MCP?

The Model Context Protocol (MCP) is an open standard by Anthropic that enables LLM applications to connect with external data sources and tools through a JSON-RPC 2.0 message-based architecture.

### Key Participants

```
┌─────────────────────────────────────────────────────────────┐
│                         HOST                                 │
│  (Claude Desktop, VS Code + Copilot, Custom Agent App)      │
│                                                              │
│   ┌──────────────┐     ┌──────────────┐                     │
│   │   CLIENT 1   │     │   CLIENT 2   │                     │
│   └──────┬───────┘     └──────┬───────┘                     │
└──────────┼─────────────────────┼────────────────────────────┘
           │                     │
    ┌──────▼───────┐      ┌──────▼───────┐
    │   SERVER A   │      │   SERVER B   │
    │  (sigil-mcp) │      │   (other)    │
    └──────────────┘      └──────────────┘
```

### Core Capabilities

| Server Capability | Description | Sigil Use |
|-------------------|-------------|-----------|
| **Tools** | Callable functions | Sign transactions, check disk |
| **Resources** | Readable data | Disk status, presig counts |
| **Prompts** | Message templates | Transaction flows |

### Protocol Version

Target: **MCP 2025-11-25** (latest stable)

---

## 2. Current Sigil Architecture

### Existing Components

```
sigil/
├── sigil-core       # Shared types, disk format, crypto
├── sigil-daemon     # System daemon, IPC server (Unix socket)
├── sigil-cli        # Claude Code tools interface
├── sigil-mother     # Air-gapped mother device tools
├── sigil-frost      # FROST threshold signatures
└── sigil-zkvm       # SP1 zero-knowledge proofs
```

### Current IPC Protocol

**Transport**: Unix domain socket (`/tmp/sigil.sock`)
**Format**: JSON-line (newline-delimited JSON)
**Pattern**: Request/response (no streaming)

```rust
// Current request types
enum IpcRequest {
    Ping,
    GetDiskStatus,
    Sign { message_hash, chain_id, description },
    UpdateTxHash { presig_index, tx_hash },
    ListChildren,
    GetPresigCount,
}
```

### Limitations of Current Design

1. **Claude-specific**: Skills only work in Claude Code
2. **Local only**: Unix socket limits to single machine
3. **No capability negotiation**: All-or-nothing access
4. **No standardization**: Custom protocol requires custom clients

---

## 3. Integration Architecture

### Proposed Architecture

```
                    ┌─────────────────────────────────────┐
                    │           AI AGENT HOST              │
                    │  (Claude Desktop, VS Code, etc.)    │
                    │                                      │
                    │         ┌─────────────────┐         │
                    │         │   MCP CLIENT    │         │
                    │         └────────┬────────┘         │
                    └──────────────────┼──────────────────┘
                                       │
                    ┌──────────────────┼──────────────────┐
                    │           TRANSPORT LAYER            │
                    │                                      │
                    │  ┌─────────┐    OR    ┌──────────┐  │
                    │  │  stdio  │          │ HTTP+SSE │  │
                    │  └─────────┘          └──────────┘  │
                    └──────────────────┼──────────────────┘
                                       │
┌──────────────────────────────────────▼──────────────────────────────────────┐
│                              SIGIL-MCP SERVER                                │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                         MCP PROTOCOL LAYER                             │  │
│  │                                                                        │  │
│  │   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐                │  │
│  │   │   TOOLS     │   │  RESOURCES  │   │   PROMPTS   │                │  │
│  │   │             │   │             │   │             │                │  │
│  │   │ • sign_tx   │   │ • disk://   │   │ • sign_evm  │                │  │
│  │   │ • check_dsk │   │   status    │   │ • sign_btc  │                │  │
│  │   │ • frost_sgn │   │ • presig:// │   │ • dkg_init  │                │  │
│  │   │ • get_addr  │   │   count     │   │             │                │  │
│  │   └─────────────┘   └─────────────┘   └─────────────┘                │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                       │                                      │
│  ┌────────────────────────────────────▼────────────────────────────────┐    │
│  │                       SIGIL CORE LAYER                               │    │
│  │                                                                      │    │
│  │   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐              │    │
│  │   │ DiskWatcher │   │ AgentStore  │   │   Signer    │              │    │
│  │   └─────────────┘   └─────────────┘   └─────────────┘              │    │
│  │                                                                      │    │
│  │   ┌─────────────┐   ┌─────────────┐                                 │    │
│  │   │ sigil-frost │   │ sigil-zkvm  │                                 │    │
│  │   └─────────────┘   └─────────────┘                                 │    │
│  └──────────────────────────────────────────────────────────────────────┘    │
│                                       │                                      │
└───────────────────────────────────────┼──────────────────────────────────────┘
                                        │
                              ┌─────────▼─────────┐
                              │   PHYSICAL DISK   │
                              │   (SIGIL floppy)  │
                              └───────────────────┘
```

### Two Deployment Modes

#### Mode 1: Standalone MCP Server (Recommended)

```bash
# Start as stdio server (for Claude Desktop, etc.)
sigil-mcp --transport stdio

# Start as HTTP server (for web clients, remote agents)
sigil-mcp --transport http --port 3000
```

The MCP server directly uses sigil-core components.

#### Mode 2: MCP Gateway to Existing Daemon

```bash
# MCP server connects to existing daemon
sigil-mcp --backend daemon --socket /tmp/sigil.sock
```

Useful for running MCP alongside existing CLI tools.

---

## 4. New Crate: sigil-mcp

### Crate Structure

```
crates/sigil-mcp/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API
│   ├── main.rs             # CLI entry point
│   ├── server.rs           # MCP server implementation
│   ├── protocol/
│   │   ├── mod.rs
│   │   ├── jsonrpc.rs      # JSON-RPC 2.0 types
│   │   ├── lifecycle.rs    # Initialize, shutdown
│   │   ├── capabilities.rs # Capability negotiation
│   │   └── messages.rs     # Request/response types
│   ├── transport/
│   │   ├── mod.rs
│   │   ├── stdio.rs        # stdin/stdout transport
│   │   └── http.rs         # HTTP + SSE transport
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── tools.rs        # tools/list, tools/call
│   │   ├── resources.rs    # resources/list, resources/read
│   │   └── prompts.rs      # prompts/list, prompts/get
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── sign_transaction.rs
│   │   ├── check_disk.rs
│   │   ├── frost_sign.rs
│   │   ├── get_address.rs
│   │   └── estimate_gas.rs
│   ├── resources/
│   │   ├── mod.rs
│   │   ├── disk_status.rs
│   │   └── presig_info.rs
│   └── prompts/
│       ├── mod.rs
│       ├── evm_transfer.rs
│       ├── bitcoin_send.rs
│       └── multi_sign.rs
```

### Dependencies

```toml
[package]
name = "sigil-mcp"
version = "0.1.0"
edition = "2021"

[dependencies]
# Sigil core
sigil-core = { path = "../sigil-core" }
sigil-frost = { path = "../sigil-frost" }

# Async runtime
tokio = { version = "1.35", features = ["full"] }

# JSON-RPC and serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# HTTP server for streamable HTTP transport
axum = { version = "0.7", features = ["http2"] }
axum-extra = { version = "0.9", features = ["typed-header"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# SSE support
tokio-stream = "0.1"
async-stream = "0.3"

# JSON Schema generation
schemars = "0.8"

# CLI
clap = { version = "4.4", features = ["derive"] }

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Disk detection
tokio-udev = "0.9"

# UUID for session IDs
uuid = { version = "1.0", features = ["v4"] }
```

---

## 5. Tool Definitions

### Overview

Tools are the primary way agents interact with Sigil. Each tool maps to a specific signing operation.

### Tool: `sigil_check_disk`

**Purpose**: Check if a signing disk is inserted and valid.

```json
{
  "name": "sigil_check_disk",
  "title": "Check Sigil Disk Status",
  "description": "Check if a Sigil signing disk is inserted, valid, and has remaining presignatures. Call this before any signing operation.",
  "inputSchema": {
    "type": "object",
    "properties": {},
    "additionalProperties": false
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "detected": { "type": "boolean", "description": "Whether a disk is inserted" },
      "child_id": { "type": "string", "description": "Short ID of the child disk" },
      "scheme": { "type": "string", "enum": ["ecdsa", "taproot", "ed25519", "ristretto255"] },
      "presigs_remaining": { "type": "integer", "description": "Number of signatures remaining" },
      "presigs_total": { "type": "integer", "description": "Total presignatures on disk" },
      "days_until_expiry": { "type": "integer", "description": "Days until disk expires" },
      "is_valid": { "type": "boolean", "description": "Whether disk passes validation" }
    }
  },
  "annotations": {
    "readOnlyHint": true,
    "openWorldHint": false
  }
}
```

### Tool: `sigil_sign_evm`

**Purpose**: Sign an EVM-compatible transaction (Ethereum, Polygon, Arbitrum, etc.)

```json
{
  "name": "sigil_sign_evm",
  "title": "Sign EVM Transaction",
  "description": "Sign a transaction hash for EVM-compatible chains using ECDSA. Requires a valid Sigil disk with remaining presignatures.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "message_hash": {
        "type": "string",
        "pattern": "^0x[a-fA-F0-9]{64}$",
        "description": "32-byte transaction hash to sign (hex with 0x prefix)"
      },
      "chain_id": {
        "type": "integer",
        "minimum": 1,
        "description": "EIP-155 chain ID (1=Ethereum, 137=Polygon, 42161=Arbitrum)"
      },
      "description": {
        "type": "string",
        "maxLength": 256,
        "description": "Human-readable description for audit log"
      }
    },
    "required": ["message_hash", "chain_id", "description"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "signature": { "type": "string", "description": "Full signature (hex)" },
      "v": { "type": "integer", "description": "Recovery parameter" },
      "r": { "type": "string", "description": "R component (hex)" },
      "s": { "type": "string", "description": "S component (hex)" },
      "presig_index": { "type": "integer", "description": "Index of presig used" },
      "proof_hash": { "type": "string", "description": "ZK proof hash (hex)" }
    }
  },
  "annotations": {
    "destructiveHint": true,
    "idempotentHint": false,
    "openWorldHint": false
  }
}
```

### Tool: `sigil_sign_frost`

**Purpose**: Sign using FROST threshold signatures (Bitcoin Taproot, Solana, Zcash)

```json
{
  "name": "sigil_sign_frost",
  "title": "Sign with FROST",
  "description": "Sign a message using FROST threshold signatures. Supports Taproot (Bitcoin), Ed25519 (Solana/Cosmos), and Ristretto255 (Zcash).",
  "inputSchema": {
    "type": "object",
    "properties": {
      "scheme": {
        "type": "string",
        "enum": ["taproot", "ed25519", "ristretto255"],
        "description": "FROST signature scheme to use"
      },
      "message_hash": {
        "type": "string",
        "pattern": "^0x[a-fA-F0-9]+$",
        "description": "Message hash to sign (hex with 0x prefix)"
      },
      "description": {
        "type": "string",
        "maxLength": 256,
        "description": "Human-readable description for audit log"
      }
    },
    "required": ["scheme", "message_hash", "description"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "scheme": { "type": "string" },
      "signature": { "type": "string", "description": "FROST signature (hex)" },
      "signature_length": { "type": "integer", "description": "Signature length in bytes" },
      "presig_index": { "type": "integer" }
    }
  },
  "annotations": {
    "destructiveHint": true,
    "idempotentHint": false,
    "openWorldHint": false
  }
}
```

### Tool: `sigil_get_address`

**Purpose**: Get the public address for the current disk

```json
{
  "name": "sigil_get_address",
  "title": "Get Signing Address",
  "description": "Get the blockchain address associated with the current Sigil disk.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "scheme": {
        "type": "string",
        "enum": ["ecdsa", "taproot", "ed25519", "ristretto255"],
        "description": "Signature scheme (defaults to disk's native scheme)"
      },
      "format": {
        "type": "string",
        "enum": ["hex", "evm", "bitcoin", "solana", "cosmos"],
        "default": "hex",
        "description": "Address format to return"
      },
      "cosmos_prefix": {
        "type": "string",
        "description": "Bech32 prefix for Cosmos chains (e.g., 'cosmos', 'osmo')"
      }
    }
  },
  "annotations": {
    "readOnlyHint": true
  }
}
```

### Tool: `sigil_update_tx_hash`

**Purpose**: Record the broadcast transaction hash for audit

```json
{
  "name": "sigil_update_tx_hash",
  "title": "Update Transaction Hash",
  "description": "After broadcasting a transaction, record the actual tx hash in the disk's audit log.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "presig_index": {
        "type": "integer",
        "description": "Presig index from the signing response"
      },
      "tx_hash": {
        "type": "string",
        "pattern": "^0x[a-fA-F0-9]{64}$",
        "description": "Actual transaction hash after broadcast"
      }
    },
    "required": ["presig_index", "tx_hash"]
  },
  "annotations": {
    "destructiveHint": false,
    "idempotentHint": true
  }
}
```

### Complete Tool List

| Tool Name | Purpose | Scheme |
|-----------|---------|--------|
| `sigil_check_disk` | Check disk status | All |
| `sigil_sign_evm` | Sign EVM transaction | ECDSA |
| `sigil_sign_frost` | Sign with FROST | Taproot/Ed25519/Ristretto |
| `sigil_get_address` | Get signing address | All |
| `sigil_update_tx_hash` | Record tx hash | All |
| `sigil_list_schemes` | List supported schemes | N/A |
| `sigil_get_presig_count` | Get remaining presigs | All |

---

## 6. Resource Definitions

Resources provide readable context data to agents.

### Resource: `sigil://disk/status`

**Purpose**: Real-time disk status (subscribable)

```json
{
  "uri": "sigil://disk/status",
  "name": "Disk Status",
  "title": "Current Sigil Disk Status",
  "description": "Real-time status of the inserted signing disk including validity, remaining presignatures, and expiry information.",
  "mimeType": "application/json"
}
```

**Content when read:**
```json
{
  "detected": true,
  "child_id": "7a3f2c1b",
  "scheme": "ecdsa",
  "presigs_remaining": 847,
  "presigs_total": 1000,
  "days_until_expiry": 12,
  "is_valid": true,
  "public_key": "0x04abc123...",
  "addresses": {
    "evm": "0x742d35Cc6634C0532925a3b844Bc9e7595f...",
    "bitcoin_legacy": "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"
  }
}
```

### Resource: `sigil://presigs/info`

**Purpose**: Detailed presignature statistics

```json
{
  "uri": "sigil://presigs/info",
  "name": "Presignature Info",
  "description": "Detailed information about presignature consumption and availability.",
  "mimeType": "application/json"
}
```

### Resource Template: `sigil://children/{child_id}`

**Purpose**: Access specific child disk information

```json
{
  "uriTemplate": "sigil://children/{child_id}",
  "name": "Child Disk Info",
  "description": "Information about a specific child disk by ID"
}
```

### Resource: `sigil://supported-chains`

**Purpose**: List of supported blockchains and their chain IDs

```json
{
  "uri": "sigil://supported-chains",
  "name": "Supported Chains",
  "description": "List of blockchain networks supported for signing",
  "mimeType": "application/json"
}
```

**Content:**
```json
{
  "evm_chains": [
    { "name": "Ethereum Mainnet", "chain_id": 1, "symbol": "ETH" },
    { "name": "Polygon", "chain_id": 137, "symbol": "MATIC" },
    { "name": "Arbitrum One", "chain_id": 42161, "symbol": "ETH" },
    { "name": "Optimism", "chain_id": 10, "symbol": "ETH" },
    { "name": "Base", "chain_id": 8453, "symbol": "ETH" }
  ],
  "frost_chains": {
    "taproot": ["Bitcoin Mainnet", "Bitcoin Testnet"],
    "ed25519": ["Solana", "Cosmos Hub", "Osmosis", "Near"],
    "ristretto255": ["Zcash (shielded)"]
  }
}
```

---

## 7. Prompt Definitions

Prompts provide guided workflows for common operations.

### Prompt: `sign_evm_transfer`

**Purpose**: Guide through EVM token transfer signing

```json
{
  "name": "sign_evm_transfer",
  "title": "Sign EVM Transfer",
  "description": "Guided workflow for signing an EVM transfer transaction",
  "arguments": [
    {
      "name": "to_address",
      "description": "Recipient address",
      "required": true
    },
    {
      "name": "amount",
      "description": "Amount to transfer (in native units)",
      "required": true
    },
    {
      "name": "chain_id",
      "description": "Chain ID (default: 1 for Ethereum)",
      "required": false
    }
  ]
}
```

**Messages returned:**
```json
{
  "messages": [
    {
      "role": "user",
      "content": {
        "type": "text",
        "text": "Sign an EVM transfer of {amount} to {to_address} on chain {chain_id}.\n\n## Pre-flight Checks\n1. Check disk status using sigil_check_disk\n2. Verify presignatures remaining\n3. Get sender address using sigil_get_address\n\n## Transaction Details\n- To: {to_address}\n- Amount: {amount}\n- Chain: {chain_id}\n\n## Signing Steps\n1. Build unsigned transaction with proper nonce and gas\n2. Compute keccak256 hash of RLP-encoded transaction\n3. Call sigil_sign_evm with the hash\n4. Combine signature with transaction\n5. Broadcast to network\n6. Call sigil_update_tx_hash with result"
      }
    }
  ]
}
```

### Prompt: `sign_bitcoin_taproot`

**Purpose**: Guide through Bitcoin Taproot transaction signing

### Prompt: `multi_signature_batch`

**Purpose**: Guide through signing multiple transactions efficiently

### Complete Prompt List

| Prompt Name | Purpose |
|-------------|---------|
| `sign_evm_transfer` | EVM native token transfer |
| `sign_erc20_transfer` | ERC-20 token transfer |
| `sign_bitcoin_taproot` | Bitcoin Taproot transaction |
| `sign_solana_transfer` | Solana SOL transfer |
| `sign_cosmos_delegate` | Cosmos staking delegation |
| `multi_signature_batch` | Multiple signatures efficiently |
| `troubleshoot_disk` | Diagnose disk issues |

---

## 8. Transport Implementation

### 8.1 stdio Transport

The primary transport for local MCP servers.

```rust
// src/transport/stdio.rs

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct StdioTransport {
    reader: BufReader<tokio::io::Stdin>,
    writer: tokio::io::Stdout,
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(tokio::io::stdin()),
            writer: tokio::io::stdout(),
        }
    }

    pub async fn read_message(&mut self) -> Result<JsonRpcMessage> {
        let mut line = String::new();
        self.reader.read_line(&mut line).await?;
        Ok(serde_json::from_str(&line)?)
    }

    pub async fn write_message(&mut self, msg: &JsonRpcMessage) -> Result<()> {
        let json = serde_json::to_string(msg)?;
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }
}
```

### 8.2 Streamable HTTP Transport

For remote and web-based agents.

```rust
// src/transport/http.rs

use axum::{
    extract::{State, Json},
    response::sse::{Event, Sse},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct HttpTransport {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    port: u16,
}

// POST /mcp - Receive JSON-RPC requests
async fn handle_post(
    State(state): State<Arc<McpServer>>,
    headers: HeaderMap,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let session_id = headers
        .get("MCP-Session-Id")
        .and_then(|v| v.to_str().ok());

    // Handle request and return SSE stream or single JSON response
    // ...
}

// GET /mcp - SSE stream for server-initiated messages
async fn handle_get(
    State(state): State<Arc<McpServer>>,
    headers: HeaderMap,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Return SSE stream for notifications
    // ...
}

// DELETE /mcp - Terminate session
async fn handle_delete(
    State(state): State<Arc<McpServer>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Terminate session
    // ...
}

pub fn create_router(server: Arc<McpServer>) -> Router {
    Router::new()
        .route("/mcp", post(handle_post))
        .route("/mcp", get(handle_get))
        .route("/mcp", delete(handle_delete))
        .with_state(server)
}
```

### 8.3 Session Management

```rust
pub struct Session {
    id: String,
    protocol_version: String,
    client_capabilities: ClientCapabilities,
    created_at: Instant,
    last_activity: Instant,
}

impl Session {
    pub fn new(protocol_version: String, capabilities: ClientCapabilities) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            id,
            protocol_version,
            client_capabilities: capabilities,
            created_at: Instant::now(),
            last_activity: Instant::now(),
        }
    }
}
```

---

## 9. Security Considerations

### 9.1 MCP-Specific Security

#### Origin Validation (HTTP Transport)

```rust
async fn validate_origin(headers: &HeaderMap) -> Result<(), McpError> {
    if let Some(origin) = headers.get("Origin") {
        let origin_str = origin.to_str()?;
        // Only allow localhost origins by default
        if !origin_str.starts_with("http://localhost")
           && !origin_str.starts_with("http://127.0.0.1") {
            return Err(McpError::ForbiddenOrigin);
        }
    }
    Ok(())
}
```

#### DNS Rebinding Protection

```rust
// Bind to localhost only, never 0.0.0.0
let addr = SocketAddr::from(([127, 0, 0, 1], port));
```

#### Session Token Security

```rust
// Generate secure session IDs
fn generate_session_id() -> String {
    // Use cryptographically secure random bytes
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill(&mut bytes);
    base64::encode_config(&bytes, base64::URL_SAFE_NO_PAD)
}
```

### 9.2 Tool Safety Annotations

All signing tools include safety annotations:

```rust
pub struct ToolAnnotations {
    /// Tool has destructive/irreversible effects (consumes presig)
    pub destructive_hint: Option<bool>,

    /// Tool is idempotent (safe to retry)
    pub idempotent_hint: Option<bool>,

    /// Tool requires no user confirmation
    pub open_world_hint: Option<bool>,
}

// For signing tools:
ToolAnnotations {
    destructive_hint: Some(true),   // Consumes presig
    idempotent_hint: Some(false),   // Each call uses new presig
    open_world_hint: Some(false),   // Requires disk insertion
}
```

### 9.3 Rate Limiting

```rust
pub struct RateLimiter {
    /// Max signing requests per minute
    max_signs_per_minute: u32,

    /// Current window count
    window_count: AtomicU32,

    /// Window start time
    window_start: RwLock<Instant>,
}

impl RateLimiter {
    pub fn check_sign_request(&self) -> Result<(), McpError> {
        let count = self.window_count.fetch_add(1, Ordering::SeqCst);
        if count > self.max_signs_per_minute {
            return Err(McpError::RateLimited);
        }
        Ok(())
    }
}
```

### 9.4 Audit Logging

```rust
#[derive(Serialize)]
pub struct AuditEvent {
    timestamp: DateTime<Utc>,
    event_type: AuditEventType,
    client_info: Option<ClientInfo>,
    tool_name: Option<String>,
    arguments: Option<serde_json::Value>,
    result: Option<AuditResult>,
}

enum AuditEventType {
    SessionStart,
    SessionEnd,
    ToolCall,
    ToolResult,
    Error,
}
```

---

## 10. Implementation Phases

### Phase 1: Core MCP Infrastructure (Week 1-2)

**Deliverables:**
- [ ] Create `sigil-mcp` crate structure
- [ ] Implement JSON-RPC 2.0 message types
- [ ] Implement lifecycle (initialize, initialized, shutdown)
- [ ] Implement capability negotiation
- [ ] Basic stdio transport
- [ ] Unit tests for protocol layer

**Key Files:**
```
src/protocol/jsonrpc.rs
src/protocol/lifecycle.rs
src/protocol/capabilities.rs
src/transport/stdio.rs
```

### Phase 2: Tool Handlers (Week 2-3)

**Deliverables:**
- [ ] Implement `tools/list` handler
- [ ] Implement `tools/call` handler
- [ ] Port `sigil_check_disk` tool
- [ ] Port `sigil_sign_evm` tool
- [ ] Port `sigil_sign_frost` tool
- [ ] Port `sigil_get_address` tool
- [ ] Integration tests with mock disk

**Key Files:**
```
src/handlers/tools.rs
src/tools/*.rs
```

### Phase 3: Resources & Prompts (Week 3-4)

**Deliverables:**
- [ ] Implement `resources/list` handler
- [ ] Implement `resources/read` handler
- [ ] Implement `prompts/list` handler
- [ ] Implement `prompts/get` handler
- [ ] Define all resource URIs
- [ ] Define all prompt templates

**Key Files:**
```
src/handlers/resources.rs
src/handlers/prompts.rs
src/resources/*.rs
src/prompts/*.rs
```

### Phase 4: HTTP Transport (Week 4-5)

**Deliverables:**
- [ ] HTTP POST handler
- [ ] SSE stream support
- [ ] Session management
- [ ] Protocol version header handling
- [ ] Origin validation
- [ ] CORS configuration

**Key Files:**
```
src/transport/http.rs
src/server.rs
```

### Phase 5: Production Hardening (Week 5-6)

**Deliverables:**
- [ ] Rate limiting
- [ ] Audit logging
- [ ] Metrics/telemetry
- [ ] Error recovery
- [ ] Documentation
- [ ] Integration tests with real agents

**Key Files:**
```
src/security/*.rs
documentation/MCP_SERVER.md
```

### Phase 6: Claude Desktop Integration (Week 6)

**Deliverables:**
- [ ] Claude Desktop configuration
- [ ] Installation script
- [ ] User documentation
- [ ] Demo video/walkthrough

---

## 11. Testing Strategy

### 11.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_request_parsing() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }"#;

        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.method, "initialize");
    }

    #[test]
    fn test_tool_schema_generation() {
        let tool = SignEvmTool::definition();
        assert_eq!(tool.name, "sigil_sign_evm");
        assert!(tool.input_schema.is_object());
    }
}
```

### 11.2 Integration Tests

```rust
#[tokio::test]
async fn test_full_signing_flow() {
    // Start MCP server
    let server = McpServer::new_test().await;

    // Initialize
    let init_response = server.handle_initialize(test_init_request()).await;
    assert!(init_response.is_ok());

    // List tools
    let tools = server.handle_tools_list().await;
    assert!(tools.iter().any(|t| t.name == "sigil_check_disk"));

    // Check disk (with mock)
    let result = server.handle_tools_call("sigil_check_disk", json!({})).await;
    assert!(result.content[0].text.contains("detected"));
}
```

### 11.3 Conformance Tests

Use MCP Inspector or similar tools:

```bash
# Test with MCP Inspector
npx @anthropic/mcp-inspector sigil-mcp --transport stdio
```

### 11.4 End-to-End Tests

```bash
# Test with Claude Desktop
claude-desktop --mcp-server sigil-mcp
```

---

## 12. Migration Path

### For Existing Claude Code Users

The existing `.claude/skills/` will continue to work. MCP provides an additional integration path.

### Configuration for Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "sigil": {
      "command": "sigil-mcp",
      "args": ["--transport", "stdio"],
      "env": {
        "SIGIL_AGENT_STORE": "/var/lib/sigil/agent_store"
      }
    }
  }
}
```

### Configuration for Other MCP Clients

**VS Code with Continue:**
```json
{
  "continue.mcpServers": {
    "sigil": {
      "command": "sigil-mcp",
      "args": ["--transport", "stdio"]
    }
  }
}
```

**HTTP-based Clients:**
```bash
# Start HTTP server
sigil-mcp --transport http --port 3000

# Connect clients to http://localhost:3000/mcp
```

---

## Appendix A: JSON-RPC Message Types

### Request

```rust
#[derive(Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,  // Always "2.0"
    pub id: RequestId,    // String or Number
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}
```

### Response

```rust
#[derive(Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}
```

### Notification

```rust
#[derive(Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}
```

---

## Appendix B: Error Codes

| Code | Name | Description |
|------|------|-------------|
| -32700 | Parse Error | Invalid JSON |
| -32600 | Invalid Request | Not valid JSON-RPC |
| -32601 | Method Not Found | Method doesn't exist |
| -32602 | Invalid Params | Invalid method params |
| -32603 | Internal Error | Server error |
| -32002 | Resource Not Found | Resource doesn't exist |

### Sigil-Specific Errors

| Code | Name | Description |
|------|------|-------------|
| -32100 | No Disk Detected | Signing disk not inserted |
| -32101 | Disk Expired | Presignatures expired |
| -32102 | No Presigs | All presigs consumed |
| -32103 | Scheme Mismatch | Wrong signature scheme |
| -32104 | Signing Failed | MPC signing error |
| -32105 | Rate Limited | Too many requests |

---

## Appendix C: Capability Matrix

### Client Capabilities (What Sigil Uses)

| Capability | Required | Usage |
|------------|----------|-------|
| `roots` | No | Not used |
| `sampling` | No | Not used |
| `elicitation` | No | Future: disk insertion prompts |

### Server Capabilities (What Sigil Provides)

| Capability | Provided | Description |
|------------|----------|-------------|
| `tools` | Yes | Signing operations |
| `tools.listChanged` | Yes | Disk insertion events |
| `resources` | Yes | Disk status, chain info |
| `resources.subscribe` | Yes | Real-time disk status |
| `resources.listChanged` | Yes | Disk insertion events |
| `prompts` | Yes | Guided workflows |
| `prompts.listChanged` | No | Static prompts |
| `logging` | Yes | Audit logs |

---

## Appendix D: References

1. [MCP Specification 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25)
2. [MCP TypeScript Schema](https://github.com/modelcontextprotocol/specification/tree/main/schema)
3. [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
4. [RFC 6570 - URI Templates](https://tools.ietf.org/html/rfc6570)
5. [Sigil Architecture Documentation](/documentation/ARCHITECTURE.md)

---

*Document Version: 1.0*
*Last Updated: 2026-01-18*
*Author: Claude (Opus 4.5)*

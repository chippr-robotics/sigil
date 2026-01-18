# sigil-mcp

MCP (Model Context Protocol) server for Sigil MPC signing.

This crate implements the [Model Context Protocol](https://modelcontextprotocol.io/) specification, enabling any MCP-compatible AI agent to securely sign blockchain transactions using Sigil's MPC infrastructure.

## Features

- **MCP 2025-11-25** protocol compliance
- **stdio transport** for local integration (Claude Desktop, VS Code)
- **7 signing tools** (EVM, FROST Taproot/Ed25519/Ristretto255)
- **4 resources** (disk status, presig info, chain info)
- **5 guided prompts** (transfer workflows, troubleshooting)
- **84 tests** (unit, integration, doc tests)

## Quick Start

### Build

```bash
cargo build -p sigil-mcp --release
```

### Run

```bash
# Start with stdio transport
sigil-mcp --transport stdio

# With debug logging
sigil-mcp --transport stdio --log-level debug

# With mock disk (for testing)
sigil-mcp --transport stdio --mock
```

## Claude Desktop Integration

Add to your Claude Desktop configuration:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
**Linux**: `~/.config/claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "sigil": {
      "command": "/usr/local/bin/sigil-mcp",
      "args": ["--transport", "stdio"]
    }
  }
}
```

## Tools

| Tool | Description | Scheme |
|------|-------------|--------|
| `sigil_check_disk` | Check if signing disk is inserted and valid | All |
| `sigil_sign_evm` | Sign EVM transactions (Ethereum, Polygon, etc.) | ECDSA |
| `sigil_sign_frost` | Sign with FROST (Bitcoin, Solana, Zcash) | Taproot/Ed25519/Ristretto |
| `sigil_get_address` | Get signing address in various formats | All |
| `sigil_update_tx_hash` | Record tx hash in audit log | All |
| `sigil_list_schemes` | List supported signature schemes | N/A |
| `sigil_get_presig_count` | Get remaining presignatures | All |

### Example Tool Call

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "sigil_sign_evm",
    "arguments": {
      "message_hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
      "chain_id": 1,
      "description": "Transfer 0.1 ETH to vitalik.eth"
    }
  }
}
```

## Resources

| URI | Description |
|-----|-------------|
| `sigil://disk/status` | Real-time status of inserted signing disk |
| `sigil://presigs/info` | Presignature consumption statistics |
| `sigil://supported-chains` | List of supported blockchain networks |
| `sigil://children/{id}` | Information about specific child disk |

## Prompts

| Prompt | Description |
|--------|-------------|
| `sign_evm_transfer` | Guided EVM token transfer workflow |
| `sign_bitcoin_taproot` | Guided Bitcoin Taproot signing |
| `sign_solana_transfer` | Guided Solana SOL transfer |
| `troubleshoot_disk` | Diagnose and resolve disk issues |
| `check_signing_readiness` | Verify system is ready to sign |

## Architecture

```
┌─────────────────────────────────────────────┐
│              AI AGENT HOST                  │
│  (Claude Desktop, VS Code, Custom Agent)   │
│                    │                        │
│         ┌─────────▼─────────┐              │
│         │    MCP CLIENT     │              │
│         └─────────┬─────────┘              │
└───────────────────┼─────────────────────────┘
                    │ stdio (JSON-RPC 2.0)
┌───────────────────▼─────────────────────────┐
│            SIGIL-MCP SERVER                 │
│                                             │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐       │
│  │  Tools  │ │Resources│ │ Prompts │       │
│  └────┬────┘ └────┬────┘ └────┬────┘       │
│       └───────────┼───────────┘             │
│                   │                         │
│  ┌────────────────▼────────────────┐       │
│  │         Sigil Core Layer        │       │
│  │  DiskState • ToolContext • MPC  │       │
│  └────────────────┬────────────────┘       │
└───────────────────┼─────────────────────────┘
                    │
          ┌─────────▼─────────┐
          │   SIGIL DISK      │
          │   (USB Floppy)    │
          └───────────────────┘
```

## Protocol Compliance

This implementation follows MCP specification 2025-11-25:

- **JSON-RPC 2.0** message format
- **Capability negotiation** (tools, resources, prompts, logging)
- **Lifecycle management** (initialize, initialized notification)
- **Error codes** (standard + Sigil-specific)
- **Tool annotations** (destructive, idempotent, read-only hints)

## Testing

```bash
# Run all tests
cargo test -p sigil-mcp

# Run only integration tests
cargo test -p sigil-mcp --test protocol_integration

# Run with output
cargo test -p sigil-mcp -- --nocapture
```

## CLI Options

```
sigil-mcp [OPTIONS]

Options:
  -t, --transport <TRANSPORT>  Transport mechanism [default: stdio]
                               Possible values: stdio, http
  -p, --port <PORT>           Port for HTTP transport [default: 3000]
  -v, --verbose               Enable verbose logging (to stderr)
      --mock                  Use mock disk state (for testing)
      --log-level <LEVEL>     Log level [default: info]
                               Possible values: trace, debug, info, warn, error
  -h, --help                  Print help
  -V, --version               Print version
```

## Security

- **Input validation**: All tool parameters are validated
- **Invariant checks**: Runtime verification of protocol compliance
- **No file system access**: Resources are in-memory only
- **Presignature consumption**: Each signature uses exactly one presig
- **Audit logging**: All operations logged for reconciliation

## Development

### Module Structure

```
src/
├── lib.rs           # Public API
├── main.rs          # CLI entry point
├── server.rs        # MCP server implementation
├── protocol/        # JSON-RPC and MCP types
│   ├── jsonrpc.rs   # JSON-RPC 2.0 types
│   ├── lifecycle.rs # Initialize, shutdown
│   ├── capabilities.rs
│   └── messages.rs  # Tools, resources, prompts types
├── transport/
│   └── stdio.rs     # stdin/stdout transport
├── handlers/
│   └── mod.rs       # Request handlers
├── tools/           # Tool implementations
│   ├── check_disk.rs
│   ├── sign_evm.rs
│   ├── sign_frost.rs
│   ├── get_address.rs
│   └── update_tx_hash.rs
├── resources/
│   └── mod.rs       # Resource handlers
├── prompts/
│   └── mod.rs       # Prompt templates
└── invariants/
    └── mod.rs       # Validation and invariants
```

## License

Apache-2.0

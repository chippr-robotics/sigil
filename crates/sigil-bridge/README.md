# sigil-bridge

HTTP bridge server that enables mobile apps to communicate with `sigil-daemon`.

## Overview

The `sigil-bridge` server provides an HTTP REST API that proxies requests to the `sigil-daemon`'s IPC interface (Unix socket or Windows named pipe). This allows the Sigil mobile app to perform signing operations remotely.

## Architecture

```
┌─────────────────┐     HTTP      ┌──────────────┐      IPC       ┌──────────────┐
│   Mobile App    │ ────────────► │ sigil-bridge │ ─────────────► │ sigil-daemon │
└─────────────────┘               └──────────────┘                └──────────────┘
                                       :8080                      /tmp/sigil.sock
```

## Installation

```bash
cargo build --release -p sigil-bridge
```

## Usage

```bash
# Start with default settings
sigil-bridge

# Custom port and socket path
sigil-bridge --port 8080 --socket-path /tmp/sigil.sock

# Enable verbose logging
sigil-bridge -v
```

## API Endpoints

### Health Check
```
GET /health
```
Returns `{"status": "ok"}` if the server is running.

### Ping Daemon
```
POST /api/ping
```
Checks connection to the daemon. Returns daemon version.

### Get Disk Status
```
POST /api/disk-status
```
Returns current disk status including presignature count, validity, and expiration.

### Get Presignature Count
```
POST /api/presig-count
```
Returns remaining and total presignatures.

### Sign EVM Transaction
```
POST /api/sign
Content-Type: application/json

{
  "message_hash": "0x1234...",
  "chain_id": 1,
  "description": "Transfer 0.1 ETH"
}
```

### Sign with FROST
```
POST /api/sign-frost
Content-Type: application/json

{
  "scheme": "taproot",
  "message_hash": "0x1234...",
  "description": "Bitcoin transfer"
}
```

### Get Address
```
POST /api/address
Content-Type: application/json

{
  "format": "evm",
  "scheme": "ecdsa"
}
```

### Update Transaction Hash
```
POST /api/update-tx-hash
Content-Type: application/json

{
  "presig_index": 42,
  "tx_hash": "0xabcd..."
}
```

### List Children
```
POST /api/list-children
```

### Import Agent Shard
```
POST /api/import-agent-shard
Content-Type: application/json

{
  "agent_shard_hex": "0x..."
}
```

### Import Child Shares
```
POST /api/import-child-shares
Content-Type: application/json

{
  "shares_json": "...",
  "replace": false
}
```

### List Supported Schemes
```
GET /api/schemes
```

## Security Considerations

1. **Network Security**: Run on a private network only. Do not expose to the internet.
2. **Authentication**: Consider adding API key authentication for production use.
3. **TLS**: Use a reverse proxy (nginx, caddy) to add HTTPS.
4. **Firewall**: Restrict access to trusted IP addresses only.

## Configuration

| Flag | Default | Description |
|------|---------|-------------|
| `--host` | `0.0.0.0` | Host to bind to |
| `--port`, `-p` | `8080` | Port to bind to |
| `--socket-path` | `/tmp/sigil.sock` | Path to daemon IPC socket |
| `--verbose`, `-v` | `false` | Enable verbose logging |

## Example Setup

1. Start the daemon:
```bash
sigil-daemon
```

2. Start the bridge:
```bash
sigil-bridge --port 8080
```

3. Configure the mobile app to connect to `http://<your-ip>:8080`

4. Verify connection:
```bash
curl http://localhost:8080/health
curl -X POST http://localhost:8080/api/ping
```

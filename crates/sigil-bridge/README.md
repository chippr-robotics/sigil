# sigil-bridge

> ## ⚠️ NOT IN THE TRUSTED COMPUTING BASE
>
> `sigil-bridge` is an **out-of-TCB convenience transport**. It terminates HTTP
> and proxies to `sigil-daemon`'s IPC socket so the mobile app can reach it.
>
> - It is **not** in the workspace's `default-members`. A bare `cargo build` at
>   the repository root does not produce this binary. Build it deliberately:
>   `cargo build -p sigil-bridge`.
> - It is `publish = false` and is not part of any supported install.
> - It **never transports key material**. There are no shard import or export
>   routes and there never will be. Shards cross the air gap on physical media
>   and are imported with in-TCB tooling.
> - Sigil's security boundary is the **physically inserted disk**, not this
>   server. The daemon re-reads presignature shares from the block device on
>   every signing operation and fails closed without one. An authenticated
>   caller here can spend presignatures the operator physically inserted — it
>   cannot create a signature out of nothing.
>
> See `.specify/memory/constitution.md`, Principle I.

HTTP bridge server that enables mobile apps to communicate with `sigil-daemon`.

## Architecture

```
┌─────────────────┐   HTTP (loopback)  ┌──────────────┐      IPC       ┌──────────────┐
│   Mobile App    │ ─────────────────► │ sigil-bridge │ ─────────────► │ sigil-daemon │
│  (+ SSH tunnel) │   Bearer token     └──────────────┘                └──────────────┘
└─────────────────┘                     127.0.0.1:8080              /run/sigil/sigil.sock
```

## Installation

```bash
cargo build --release -p sigil-bridge
```

It is not installed by `scripts/install.sh` and is not produced by the default
workspace build. That is deliberate.

## Usage

```bash
# Loopback only, token generated on first run
sigil-bridge

# Supply your own token
SIGIL_BRIDGE_TOKEN='...' sigil-bridge
sigil-bridge --token-file /etc/sigil/bridge.token
```

### Reaching it from a phone

The supported path is a tunnel, not a LAN bind:

```bash
# On the phone's host / laptop
ssh -N -L 8080:127.0.0.1:8080 agent-device
```

Binding a LAN interface is possible but requires explicit acknowledgement,
because it exposes a signing endpoint to every host that can route to you:

```bash
sigil-bridge --host 0.0.0.0 --allow-non-loopback
```

Without `--allow-non-loopback`, a non-loopback `--host` is refused at startup.

## Authentication

Every `/api/*` route requires `Authorization: Bearer <token>`. Requests without
a valid token are rejected with `401` **before** any IPC call reaches the
daemon.

Token resolution order:

1. `--token-file <path>` — first line of the file.
2. `$SIGIL_BRIDGE_TOKEN`.
3. Generated at startup and written to `$XDG_RUNTIME_DIR/sigil-bridge.token`
   with mode `0600`. The path is logged.

There is no unauthenticated mode.

## CORS

None by default — no CORS headers are emitted, so browsers refuse cross-origin
responses. Grant origins explicitly if you need them:

```bash
sigil-bridge --allow-origin https://app.example
```

The previous wildcard policy (`allow_origin(Any)`) made every browser that
could route to the bridge a confused deputy for `POST /api/sign`.

## API Endpoints

`/health` is the only unauthenticated route. It reports liveness and nothing
else — no disk state, no presignature counts, no addresses, no child IDs.

```
GET /health          -> {"status":"ok"}
```

All routes below require a bearer token.

### Ping Daemon
```
POST /api/ping
```

### Get Disk Status
```
POST /api/disk-status
```

### Get Presignature Count
```
POST /api/presig-count
```

### Sign EVM Transaction
```
POST /api/sign
Content-Type: application/json
Authorization: Bearer <token>

{
  "message_hash": "0x1234...",
  "chain_id": 1,
  "description": "Transfer 0.1 ETH"
}
```

### Sign with FROST
```
POST /api/sign-frost

{
  "scheme": "taproot",
  "message_hash": "0x1234...",
  "description": "Bitcoin transfer"
}
```

### Get Address
```
POST /api/address

{
  "format": "evm",
  "scheme": "ecdsa"
}
```

### Update Transaction Hash
```
POST /api/update-tx-hash

{
  "presig_index": 42,
  "tx_hash": "0xabcd..."
}
```

### List Children
```
POST /api/list-children
```

### List Supported Schemes
```
GET /api/schemes
```

### Removed: shard import

`POST /api/import-agent-shard` and `POST /api/import-child-shares` **no longer
exist** and return `404`. They moved secret key material over HTTP. Import
shards with `sigil-cli` on the device itself.

## Security Considerations

1. **Loopback by default.** Do not bind a LAN interface; tunnel instead. If you
   must, `--allow-non-loopback` makes the exposure explicit and logged.
2. **Token required.** Keep the token file `0600`. Rotate by deleting it and
   restarting.
3. **TLS.** Loopback needs none. If you acknowledge a non-loopback bind, put a
   reverse proxy (nginx, caddy) in front and terminate HTTPS there.
4. **Firewall.** Restrict to trusted hosts even with a token.
5. **This is not the boundary.** Every one of the above is defence in depth
   around a component that is explicitly outside the TCB. Physical possession
   of the disk is the boundary.

## Configuration

| Flag | Default | Description |
|------|---------|-------------|
| `--host` | `127.0.0.1` | Host to bind to. Non-loopback requires `--allow-non-loopback`. |
| `--port`, `-p` | `8080` | Port to bind to |
| `--socket-path` | `/run/sigil/sigil.sock` | Path to daemon IPC socket |
| `--allow-non-loopback` | `false` | Acknowledge exposing signing endpoints to the network |
| `--token-file` | _(none)_ | File holding the bearer token |
| `--allow-origin` | _(none)_ | Permit a browser origin (repeatable) |
| `--verbose`, `-v` | `false` | Enable verbose logging |

## Example Setup

1. Start the daemon:
```bash
sigil-daemon
```

2. Start the bridge:
```bash
sigil-bridge
# note the logged token path, e.g. /run/user/1000/sigil-bridge.token
```

3. Tunnel from the device running the app:
```bash
ssh -N -L 8080:127.0.0.1:8080 agent-device
```

4. Verify:
```bash
curl http://127.0.0.1:8080/health
curl -X POST http://127.0.0.1:8080/api/ping \
  -H "Authorization: Bearer $(cat /run/user/1000/sigil-bridge.token)"
```

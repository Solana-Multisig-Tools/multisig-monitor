# Running Multisig Monitor Locally

This guide covers running the monitor on your local machine for development and testing. Both deployment targets (Cloudflare Worker and AWS Lambda) have local development servers that accept the same webhook payloads.

## Example Values

| Value | Example |
|---|---|
| Multisig address | `SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc` |
| Webhook auth token | `dev-token-123` |
| Slack webhook URL | `https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere` |

## Prerequisites

- [Rust](https://rustup.rs/) (stable 1.75+)
- The `wasm32-unknown-unknown` target (for the Cloudflare Worker path):
  ```sh
  rustup target add wasm32-unknown-unknown
  ```

## Option A: Cloudflare Worker (wrangler dev)

### 1. Install Wrangler

```sh
npm install -g wrangler
```

### 2. Configure wrangler.toml

Edit `wrangler.toml` at the project root. For local development, you can put secrets directly in vars (these never leave your machine):

```toml
name = "multisig-monitor"
main = "build/worker/shim.mjs"
compatibility_date = "2024-09-25"

[build]
command = "cargo install -q worker-build && worker-build --release"
cwd = "crates/worker-cf"

[vars]
MULTISIG_ADDRESSES = "SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc"
INSTRUCTION_TYPES = ""
CONFIG_TYPES = ""
HELIUS_AUTH_TOKEN = "dev-token-123"
SLACK_WEBHOOK_URL = "https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere"
```

Alternatively, create a `.dev.vars` file in the project root to keep secrets out of `wrangler.toml`:

```sh
# .dev.vars (not committed to git)
HELIUS_AUTH_TOKEN=dev-token-123
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere
```

### 3. Start the Dev Server

```sh
wrangler dev
```

The first run compiles the Rust code to WASM (takes ~30s). Subsequent starts use the cache. The server runs at:

```
http://localhost:8787
```

### 4. Test It

Health check:

```sh
curl http://localhost:8787/health
# ok
```

Send an empty webhook (returns 200, no Slack notification):

```sh
curl -X POST http://localhost:8787/webhook \
  -H "Authorization: dev-token-123" \
  -H "Content-Type: application/json" \
  -d '[{
    "signature": "test",
    "slot": 0,
    "transaction": {
      "message": { "accountKeys": [], "instructions": [] },
      "signatures": ["test"]
    },
    "meta": null
  }]'
# ok
```

Verify auth rejection:

```sh
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8787/webhook \
  -H "Authorization: wrong" \
  -d '[]'
# 500
```

## Option B: AWS Lambda (cargo lambda watch)

### 1. Install cargo-lambda

```sh
cargo install cargo-lambda
```

### 2. Start the Dev Server

Pass configuration as environment variables:

```sh
MULTISIG_ADDRESSES="SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc" \
INSTRUCTION_TYPES="" \
CONFIG_TYPES="" \
HELIUS_AUTH_TOKEN="dev-token-123" \
SLACK_WEBHOOK_URL="https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere" \
RUST_LOG=info \
cargo lambda watch -p multisig-monitor-lambda
```

The server compiles natively (no WASM) and runs at:

```
http://localhost:9000
```

### 3. Test It

Health check:

```sh
curl http://localhost:9000/health
# ok
```

Send an empty webhook:

```sh
curl -X POST http://localhost:9000/webhook \
  -H "Authorization: dev-token-123" \
  -H "Content-Type: application/json" \
  -d '[{
    "signature": "test",
    "slot": 0,
    "transaction": {
      "message": { "accountKeys": [], "instructions": [] },
      "signatures": ["test"]
    },
    "meta": null
  }]'
# ok
```

Verify auth rejection:

```sh
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:9000/webhook \
  -H "Authorization: wrong" \
  -d '[]'
# 401
```

## Testing with a Realistic Payload

To test the full parsing pipeline locally without a live Helius webhook, you can craft a payload that mimics a real Squads v4 transaction. The payload below simulates a `proposal_approve` instruction.

First, get a valid discriminator. The monitor computes these at runtime, but you can compute one with:

```sh
cargo test -p multisig-monitor-core -- discriminator --nocapture 2>&1 | head -20
```

For a more practical test, point Helius at your local machine using a tunneling tool:

### Using a Tunnel (ngrok, cloudflared, etc.)

1. Start the local server (either `wrangler dev` or `cargo lambda watch`)

2. In another terminal, expose it publicly:

   ```sh
   # Using ngrok
   ngrok http 8787    # for wrangler dev
   ngrok http 9000    # for cargo lambda watch

   # Or using cloudflared
   cloudflared tunnel --url http://localhost:8787
   ```

3. Copy the public URL (e.g., `https://abc123.ngrok-free.app`)

4. Create a Helius webhook pointing at your tunnel:

   ```sh
   curl -X POST "https://api.helius.xyz/v0/webhooks?api-key=<YOUR_API_KEY>" \
     -H "Content-Type: application/json" \
     -d '{
       "webhookURL": "https://abc123.ngrok-free.app/webhook",
       "transactionTypes": ["ANY"],
       "accountAddresses": ["SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc"],
       "webhookType": "raw",
       "authHeader": "dev-token-123"
     }'
   ```

5. Perform an action on the monitored multisig (or wait for someone else to). The webhook fires, hits your tunnel, and you see the notification in Slack and logs in your terminal.

6. When done, delete the test webhook:

   ```sh
   curl -X DELETE "https://api.helius.xyz/v0/webhooks/<WEBHOOK_ID>?api-key=<YOUR_API_KEY>"
   ```

## Running Tests

The core parsing logic is covered by unit tests that don't require any network access or configuration:

```sh
# All tests
cargo test --workspace

# Just the core parsing tests
cargo test -p multisig-monitor-core

# A specific test
cargo test -p multisig-monitor-core -- decode_direct_add_member

# With output
cargo test -p multisig-monitor-core -- --nocapture
```

## Linting

```sh
cargo clippy --workspace
```

## Verifying Both Build Targets

Confirm both deployment targets compile without deploying:

```sh
# Cloudflare Worker (WASM)
cargo build -p multisig-monitor-cf --target wasm32-unknown-unknown --release

# AWS Lambda (native)
cargo build -p multisig-monitor-lambda --release
```

## Environment Variable Reference

All variables are the same across local, Cloudflare, and Lambda environments:

| Variable | Required | Default | Description |
|---|---|---|---|
| `MULTISIG_ADDRESSES` | Yes | — | Comma-separated multisig addresses |
| `HELIUS_AUTH_TOKEN` | Yes | — | Shared secret for webhook auth |
| `SLACK_WEBHOOK_URL` | Yes | — | Slack incoming webhook URL |
| `INSTRUCTION_TYPES` | No | `""` (all) | Comma-separated instruction type filter |
| `CONFIG_TYPES` | No | `""` (all) | Comma-separated config action filter |
| `RUST_LOG` | No | — | Log level (Lambda only): `error`, `warn`, `info`, `debug`, `trace` |

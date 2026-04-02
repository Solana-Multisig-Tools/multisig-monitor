# Running the TypeScript Monitor Locally

This guide covers running the TypeScript monitor on your local machine for development and testing. Both deployment targets (Cloudflare Workers and Vercel Edge) have local development servers.

## Example Values

| Value | Example |
|---|---|
| Multisig address | `SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc` |
| Webhook auth token | `dev-token-123` |
| Slack webhook URL | `https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere` |

## Prerequisites

- [Node.js](https://nodejs.org/) 18+

## Step 1: Install Dependencies

From the `ts/` directory:

```sh
cd ts
npm install
```

## Step 2: Run Tests

```sh
npm test
```

You should see 26 tests passing. The tests run entirely in-process with no network access or configuration required.

## Option A: Cloudflare Workers (wrangler dev)

### Configure wrangler.toml

Edit `ts/wrangler.toml`. For local development, you can put secrets directly in vars (these never leave your machine):

```toml
name = "multisig-monitor"
main = "src/index.ts"
compatibility_date = "2024-09-25"

[vars]
MULTISIG_ADDRESSES = "SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc"
INSTRUCTION_TYPES = "config_transaction_create,config_transaction_execute,multisig_add_member,multisig_remove_member,multisig_change_threshold,multisig_set_time_lock,multisig_set_config_authority,multisig_set_rent_collector,multisig_add_spending_limit,multisig_remove_spending_limit"
CONFIG_TYPES = ""
HELIUS_AUTH_TOKEN = "dev-token-123"
SLACK_WEBHOOK_URL = "https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere"
```

Alternatively, create a `.dev.vars` file in the `ts/` directory to keep secrets out of `wrangler.toml`:

```sh
# ts/.dev.vars (not committed to git)
HELIUS_AUTH_TOKEN=dev-token-123
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere
```

### Start the Dev Server

```sh
npx wrangler dev
```

The server starts at:

```
http://localhost:8787
```

No compile step needed — Wrangler bundles TypeScript directly.

### Test It

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
# 401
```

## Option B: Vercel Edge (vercel dev)

### Set Environment Variables

Create a `.env.local` file in the `ts/` directory:

```sh
# ts/.env.local (not committed to git)
MULTISIG_ADDRESSES=SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc
INSTRUCTION_TYPES=config_transaction_create,config_transaction_execute,multisig_add_member,multisig_remove_member,multisig_change_threshold,multisig_set_time_lock,multisig_set_config_authority,multisig_set_rent_collector,multisig_add_spending_limit,multisig_remove_spending_limit
CONFIG_TYPES=
HELIUS_AUTH_TOKEN=dev-token-123
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere
```

### Start the Dev Server

```sh
npx vercel dev
```

The server starts at:

```
http://localhost:3000
```

### Test It

Note: Vercel routes use the `/api/` prefix.

Health check:

```sh
curl http://localhost:3000/api/health
# ok
```

Send an empty webhook:

```sh
curl -X POST http://localhost:3000/api/webhook \
  -H "Authorization: dev-token-123" \
  -H "Content-Type: application/json" \
  -d '[{
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
  -X POST http://localhost:3000/api/webhook \
  -H "Authorization: wrong" \
  -d '[]'
# 401
```

## Testing with Real Helius Data

To test the full parsing pipeline locally, expose your local server to the internet with a tunnel:

### Using a Tunnel (ngrok, cloudflared, etc.)

1. Start the local server (either `wrangler dev` or `vercel dev`)

2. In another terminal, expose it publicly:

   ```sh
   # For wrangler dev (port 8787)
   ngrok http 8787

   # For vercel dev (port 3000)
   ngrok http 3000
   ```

3. Copy the public URL (e.g., `https://abc123.ngrok-free.app`)

4. Create a Helius webhook pointing at your tunnel. Adjust the path based on which server you're running:

   ```sh
   # For wrangler dev (no /api prefix)
   curl -X POST "https://api.helius.xyz/v0/webhooks?api-key=<YOUR_API_KEY>" \
     -H "Content-Type: application/json" \
     -d '{
       "webhookURL": "https://abc123.ngrok-free.app/webhook",
       "transactionTypes": ["ANY"],
       "accountAddresses": ["SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc"],
       "webhookType": "raw",
       "authHeader": "dev-token-123"
     }'

   # For vercel dev (with /api prefix)
   curl -X POST "https://api.helius.xyz/v0/webhooks?api-key=<YOUR_API_KEY>" \
     -H "Content-Type: application/json" \
     -d '{
       "webhookURL": "https://abc123.ngrok-free.app/api/webhook",
       "transactionTypes": ["ANY"],
       "accountAddresses": ["SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc"],
       "webhookType": "raw",
       "authHeader": "dev-token-123"
     }'
   ```

5. Perform a Squads operation on the monitored multisig. You should see console output in your terminal and a notification in Slack.

6. When done, delete the test webhook:

   ```sh
   curl -X DELETE "https://api.helius.xyz/v0/webhooks/<WEBHOOK_ID>?api-key=<YOUR_API_KEY>"
   ```

## Running Tests

```sh
# All tests
npm test

# Watch mode (re-runs on file changes)
npm run test:watch

# Type checking only (no test execution)
npm run lint
```

## Environment Variable Reference

All variables are the same across local, Cloudflare Workers, and Vercel Edge:

| Variable | Required | Default | Description |
|---|---|---|---|
| `MULTISIG_ADDRESSES` | Yes | — | Comma-separated multisig addresses |
| `HELIUS_AUTH_TOKEN` | Yes | — | Shared secret you create for webhook auth |
| `SLACK_WEBHOOK_URL` | Yes | — | Slack incoming webhook URL |
| `INSTRUCTION_TYPES` | No | `""` (all) | Comma-separated instruction type filter |
| `CONFIG_TYPES` | No | `""` (all) | Comma-separated config action filter |

## Differences Between Local Servers

| | wrangler dev | vercel dev |
|---|---|---|
| Port | 8787 | 3000 |
| Webhook path | `/webhook` | `/api/webhook` |
| Health path | `/health` | `/api/health` |
| Config source | `wrangler.toml` [vars] + `.dev.vars` | `.env.local` |
| Runtime | Cloudflare Workers (V8 isolate) | Vercel Edge (Node.js-based) |
| TS handling | Bundled by Wrangler directly | Bundled by Vercel dev server |

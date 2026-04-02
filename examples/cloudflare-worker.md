# Deploying Multisig Monitor on Cloudflare Workers

This guide walks through deploying the multisig monitor as a Cloudflare Worker, configured to monitor **all config transactions** for a single Squads v4 multisig.

## Example Values

Throughout this guide we use these placeholder values:

| Value | Example |
|---|---|
| Multisig address | `SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc` |
| Helius API key | `aaa111-bb22-cc33-dd44-eee555fff666` |
| Webhook auth token | `whsec_m0n1t0r_s3cr3t_t0k3n` |
| Slack webhook URL | `https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere` |

Replace these with your actual values.

## Prerequisites

- A [Cloudflare account](https://dash.cloudflare.com/sign-up) (free plan works)
- [Node.js](https://nodejs.org/) 18+ (for Wrangler CLI)
- [Rust](https://rustup.rs/) (stable 1.75+)
- The `wasm32-unknown-unknown` target:
  ```sh
  rustup target add wasm32-unknown-unknown
  ```
- A [Helius account](https://dashboard.helius.dev/) (free tier supports webhooks)
- A [Slack incoming webhook](https://api.slack.com/messaging/webhooks)

## Step 1: Install Wrangler

```sh
npm install -g wrangler
```

Authenticate with your Cloudflare account:

```sh
wrangler login
```

## Step 2: Configure wrangler.toml

Edit the `wrangler.toml` at the project root. Set the multisig address and filter to config transactions only:

```toml
name = "multisig-monitor"
main = "build/worker/shim.mjs"
compatibility_date = "2024-09-25"

[build]
command = "cargo install -q worker-build && worker-build --release"
cwd = "crates/worker-cf"

[vars]
MULTISIG_ADDRESSES = "SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc"
INSTRUCTION_TYPES = "config_transaction_create,config_transaction_execute,multisig_add_member,multisig_remove_member,multisig_change_threshold,multisig_set_time_lock,multisig_set_config_authority,multisig_set_rent_collector,multisig_add_spending_limit,multisig_remove_spending_limit"
CONFIG_TYPES = ""
```

This configuration monitors:
- **Proposal-based config changes**: `config_transaction_create` (with decoded actions) and `config_transaction_execute`
- **Direct config changes** (controlled multisigs): all `multisig_*` config instructions that bypass proposals

`CONFIG_TYPES` is left empty to include all config action types (add member, change threshold, etc.).

## Step 3: Set Secrets

Secrets are stored encrypted in Cloudflare and never appear in `wrangler.toml`.

```sh
wrangler secret put HELIUS_AUTH_TOKEN
```

When prompted, enter: `whsec_m0n1t0r_s3cr3t_t0k3n`

```sh
wrangler secret put SLACK_WEBHOOK_URL
```

When prompted, enter: `https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere`

## Step 4: Build and Deploy

From the project root:

```sh
wrangler deploy
```

This compiles the Rust code to WASM and deploys the Worker. On success, Wrangler prints the deployed URL:

```
Published multisig-monitor (x.xxs)
  https://multisig-monitor.<your-subdomain>.workers.dev
```

Note this URL — you need it for the Helius webhook.

## Step 5: Verify the Deployment

```sh
curl https://multisig-monitor.<your-subdomain>.workers.dev/health
```

Expected response: `ok`

Test that unauthorized requests are rejected:

```sh
curl -s -o /dev/null -w "%{http_code}" \
  -X POST https://multisig-monitor.<your-subdomain>.workers.dev/webhook \
  -H "Authorization: wrong-token" \
  -d '[]'
```

Expected response: `500` (the Worker rejects invalid auth tokens).

## Step 6: Configure the Helius Webhook

### Option A: Via the Helius Dashboard

1. Go to [dashboard.helius.dev/webhooks](https://dashboard.helius.dev/webhooks)
2. Click **New Webhook**
3. Fill in:
   - **Webhook Type**: `Raw`
   - **Webhook URL**: `https://multisig-monitor.<your-subdomain>.workers.dev/webhook`
   - **Account Addresses**: `SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc`
   - **Transaction Types**: `ANY` (the Worker handles filtering)
   - **Auth Header**: `whsec_m0n1t0r_s3cr3t_t0k3n`
4. Click **Create Webhook**

### Option B: Via the Helius API

```sh
curl -X POST "https://api.helius.xyz/v0/webhooks?api-key=aaa111-bb22-cc33-dd44-eee555fff666" \
  -H "Content-Type: application/json" \
  -d '{
    "webhookURL": "https://multisig-monitor.<your-subdomain>.workers.dev/webhook",
    "transactionTypes": ["ANY"],
    "accountAddresses": ["SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc"],
    "webhookType": "raw",
    "authHeader": "whsec_m0n1t0r_s3cr3t_t0k3n"
  }'
```

Save the returned webhook ID — you can use it later to update or delete the webhook.

## Step 7: Test End-to-End

Perform any config operation on the monitored multisig (e.g., propose a threshold change via the Squads UI). Within a few seconds you should see a Slack notification like:

```
⚙️ Config Transaction Created

Multisig:      SMPL...jXc
Creator:       9xKz...mR2p
Transaction:   txAc...5kPd

Config Actions:
  • Change threshold to 3

Tx: 4fGh...view on Solscan
```

## Local Development

Run the Worker locally without deploying:

```sh
wrangler dev
```

This starts a local server at `http://localhost:8787`. You can test it with a sample payload:

```sh
curl -X POST http://localhost:8787/webhook \
  -H "Authorization: whsec_m0n1t0r_s3cr3t_t0k3n" \
  -H "Content-Type: application/json" \
  -d '[{
    "signature": "test123",
    "slot": 300000000,
    "transaction": {
      "message": {
        "accountKeys": [],
        "instructions": []
      },
      "signatures": ["test123"]
    },
    "meta": null
  }]'
```

This sends an empty transaction — the Worker will return `ok` but produce no Slack notification (no Squads instructions to parse). To generate real notifications, use an actual Helius webhook pointing at a multisig with activity.

## Updating Configuration

To change the monitored addresses or filters, edit `wrangler.toml` and redeploy:

```sh
wrangler deploy
```

To rotate the auth token or Slack URL:

```sh
wrangler secret put HELIUS_AUTH_TOKEN
wrangler secret put SLACK_WEBHOOK_URL
```

Then update the Helius webhook's auth header to match the new token.

## Monitoring and Logs

View real-time Worker logs:

```sh
wrangler tail
```

This streams `console_error!` output, which includes any Slack delivery failures.

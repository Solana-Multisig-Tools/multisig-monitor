# Deploying the TypeScript Monitor on Cloudflare Workers

This guide walks through deploying the TypeScript multisig monitor as a Cloudflare Worker, configured to monitor **all config transactions** for a single Squads v4 multisig.

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
- [Node.js](https://nodejs.org/) 18+ (for Wrangler CLI and the TypeScript build)
- A [Helius account](https://dashboard.helius.dev/) (free tier supports webhooks)
- A [Slack incoming webhook](https://api.slack.com/messaging/webhooks)

## Step 1: Install Dependencies

From the `ts/` directory:

```sh
cd ts
npm install
```

This installs the runtime dependency (`bs58`) and dev tooling (TypeScript, Wrangler, Vitest).

## Step 2: Run Tests

Verify everything works before configuring:

```sh
npm test
```

You should see 26 tests passing across 4 test files.

## Step 3: Authenticate Wrangler

If you haven't already:

```sh
npx wrangler login
```

## Step 4: Configure wrangler.toml

Edit `ts/wrangler.toml`. Set the multisig address and filter to config transactions only:

```toml
name = "multisig-monitor"
main = "src/index.ts"
compatibility_date = "2024-09-25"

[vars]
MULTISIG_ADDRESSES = "SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc"
INSTRUCTION_TYPES = "config_transaction_create,config_transaction_execute,multisig_add_member,multisig_remove_member,multisig_change_threshold,multisig_set_time_lock,multisig_set_config_authority,multisig_set_rent_collector,multisig_add_spending_limit,multisig_remove_spending_limit"
CONFIG_TYPES = ""
```

This monitors:
- **Proposal-based config changes**: `config_transaction_create` (with decoded actions) and `config_transaction_execute`
- **Direct config changes** (controlled multisigs): all `multisig_*` instructions that bypass proposals

`CONFIG_TYPES` is left empty to include all config action types.

## Step 5: Set Secrets

Secrets are stored encrypted in Cloudflare and never appear in `wrangler.toml`.

```sh
npx wrangler secret put HELIUS_AUTH_TOKEN
```

When prompted, enter: `whsec_m0n1t0r_s3cr3t_t0k3n`

```sh
npx wrangler secret put SLACK_WEBHOOK_URL
```

When prompted, enter: `https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere`

## Step 6: Deploy

```sh
npx wrangler deploy
```

Wrangler bundles the TypeScript source directly — no separate build step needed. On success it prints the deployed URL:

```
Published multisig-monitor (x.xxs)
  https://multisig-monitor.<your-subdomain>.workers.dev
```

Note this URL — you need it for the Helius webhook.

## Step 7: Verify the Deployment

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

Expected response: `401`

## Step 8: Configure the Helius Webhook

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

Save the returned webhook ID for later management.

## Step 9: Test End-to-End

Perform any config operation on the monitored multisig (e.g., propose a threshold change via the Squads UI). Within a few seconds you should see a Slack notification like:

```
⚙️ Config Transaction Created

Multisig: SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc
Creator: 9xKzmR2pLn4sHj7wBcDfAe8qYt6vXkZ3nPo1uWr5mQjS

Transaction: TxAddr1234567890abcdefghijklmnopqrstuvwxyz12

Config Actions:
  • Change threshold to 3

Tx: 5nNtjezQMYBHvgSQmoRmJPiXGsPAWmJPoGSa64xanqrauogiVz (link to Solscan)
```

## Updating Configuration

To change the monitored addresses or filters, edit `wrangler.toml` and redeploy:

```sh
npx wrangler deploy
```

To rotate secrets:

```sh
npx wrangler secret put HELIUS_AUTH_TOKEN
npx wrangler secret put SLACK_WEBHOOK_URL
```

## Monitoring and Logs

View real-time Worker logs:

```sh
npx wrangler tail
```

This streams `console.error` output, which includes any Slack delivery failures.

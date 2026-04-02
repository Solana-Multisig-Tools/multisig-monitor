# Deploying the TypeScript Monitor on Vercel Edge Functions

This guide walks through deploying the TypeScript multisig monitor as a Vercel Edge Function, configured to monitor **all config transactions** for a single Squads v4 multisig.

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

- A [Vercel account](https://vercel.com/signup) (Hobby plan works)
- [Node.js](https://nodejs.org/) 18+
- [Vercel CLI](https://vercel.com/docs/cli):
  ```sh
  npm install -g vercel
  ```
- A [Helius account](https://dashboard.helius.dev/) (free tier supports webhooks)
- A [Slack incoming webhook](https://api.slack.com/messaging/webhooks)

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

You should see 26 tests passing across 4 test files.

## Step 3: Link to Vercel

If this is a new project:

```sh
vercel link
```

Follow the prompts to create or link to a Vercel project. Choose the `ts/` directory as the project root.

## Step 4: Set Environment Variables

Vercel Edge Functions read configuration from environment variables set in the Vercel dashboard or CLI.

```sh
vercel env add MULTISIG_ADDRESSES
```

When prompted, enter: `SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc`
Select all environments (Production, Preview, Development).

```sh
vercel env add INSTRUCTION_TYPES
```

When prompted, enter:

```
config_transaction_create,config_transaction_execute,multisig_add_member,multisig_remove_member,multisig_change_threshold,multisig_set_time_lock,multisig_set_config_authority,multisig_set_rent_collector,multisig_add_spending_limit,multisig_remove_spending_limit
```

```sh
vercel env add CONFIG_TYPES
```

When prompted, leave empty (press Enter) to include all config action types.

```sh
vercel env add HELIUS_AUTH_TOKEN
```

When prompted, enter: `whsec_m0n1t0r_s3cr3t_t0k3n`
Select **Sensitive** when asked, so it's encrypted at rest.

```sh
vercel env add SLACK_WEBHOOK_URL
```

When prompted, enter: `https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere`
Select **Sensitive** when asked.

## Step 5: Deploy

```sh
vercel --prod
```

On success, Vercel prints the deployment URL:

```
✅ Production: https://multisig-monitor-xxxxx.vercel.app
```

## Step 6: Verify the Deployment

Vercel Edge Functions are routed through the `api/` directory. The health and webhook endpoints are at:

```sh
curl https://multisig-monitor-xxxxx.vercel.app/api/health
```

Expected response: `ok`

Test auth rejection:

```sh
curl -s -o /dev/null -w "%{http_code}" \
  -X POST https://multisig-monitor-xxxxx.vercel.app/api/webhook \
  -H "Authorization: wrong-token" \
  -d '[]'
```

Expected response: `401`

## Step 7: Configure the Helius Webhook

Note that Vercel Edge Function routes are prefixed with `/api/`. The webhook URL includes this prefix.

### Option A: Via the Helius Dashboard

1. Go to [dashboard.helius.dev/webhooks](https://dashboard.helius.dev/webhooks)
2. Click **New Webhook**
3. Fill in:
   - **Webhook Type**: `Raw`
   - **Webhook URL**: `https://multisig-monitor-xxxxx.vercel.app/api/webhook`
   - **Account Addresses**: `SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc`
   - **Transaction Types**: `ANY`
   - **Auth Header**: `whsec_m0n1t0r_s3cr3t_t0k3n`
4. Click **Create Webhook**

### Option B: Via the Helius API

```sh
curl -X POST "https://api.helius.xyz/v0/webhooks?api-key=aaa111-bb22-cc33-dd44-eee555fff666" \
  -H "Content-Type: application/json" \
  -d '{
    "webhookURL": "https://multisig-monitor-xxxxx.vercel.app/api/webhook",
    "transactionTypes": ["ANY"],
    "accountAddresses": ["SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc"],
    "webhookType": "raw",
    "authHeader": "whsec_m0n1t0r_s3cr3t_t0k3n"
  }'
```

## Step 8: Test End-to-End

Perform any config operation on the monitored multisig. Within a few seconds you should see a Slack notification like:

```
⚙️ Config Transaction Created

Multisig: SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc
Creator: 9xKzmR2pLn4sHj7wBcDfAe8qYt6vXkZ3nPo1uWr5mQjS

Transaction: TxAddr1234567890abcdefghijklmnopqrstuvwxyz12

Config Actions:
  • Change threshold to 3

Tx: 5nNtjezQMYBHvgSQmoRmJPiXGsPAWmJPoGSa64xanqrauogiVz (link to Solscan)
```

## URL Routing Differences from Cloudflare

The TypeScript codebase is shared between Cloudflare Workers and Vercel Edge, but the URL paths differ:

| Endpoint | Cloudflare Workers | Vercel Edge |
|---|---|---|
| Health check | `/health` | `/api/health` |
| Webhook receiver | `/webhook` | `/api/webhook` |

The `api/webhook.ts` Vercel entry point strips the `/api` prefix internally before passing to the shared handler, so the core logic is identical.

## Updating Configuration

To update environment variables:

```sh
vercel env rm INSTRUCTION_TYPES
vercel env add INSTRUCTION_TYPES
```

Then redeploy:

```sh
vercel --prod
```

## Monitoring and Logs

View function logs in the Vercel dashboard under your project's **Logs** tab, or stream them via CLI:

```sh
vercel logs https://multisig-monitor-xxxxx.vercel.app --follow
```

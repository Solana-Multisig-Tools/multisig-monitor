# Multisig Monitor

Real-time Slack notifications for [Squads Protocol v4](https://squads.so) multisig activity on Solana.

Monitors configured multisig addresses via [Helius](https://helius.dev) raw webhooks and sends rich Slack notifications when members create, vote on, or execute transactions. Decodes Squads v4 instruction data on-chain — including config transaction actions like adding members or changing thresholds — so notifications include full operational context.

## Architecture

```
Helius (raw webhook) ──POST──> Worker ──POST──> Slack Incoming Webhook
```

A stateless HTTP handler receives raw transaction data from Helius, parses Squads v4 instructions by matching Anchor discriminators, resolves the acting member from the transaction's account keys, and posts a formatted notification to Slack.

Two deployment targets share the same core parsing and notification logic:

| Crate | Target | Runtime |
|---|---|---|
| `multisig-monitor-cf` | Cloudflare Workers | WASM (`wasm32-unknown-unknown`) |
| `multisig-monitor-lambda` | AWS Lambda | Native Linux (`x86_64` or `aarch64`) |

## Project Structure

```
multisig-monitor/
├── Cargo.toml                          # Workspace root
├── wrangler.toml                       # Cloudflare Workers deployment config
├── crates/
│   ├── core/                           # Shared library (multisig-monitor-core)
│   │   └── src/
│   │       ├── error.rs                # Error types
│   │       ├── filter.rs               # Notification filtering by instruction/config type
│   │       ├── squads/                 # Squads v4 instruction parsing
│   │       │   ├── discriminator.rs    # Anchor discriminator computation
│   │       │   ├── accounts.rs         # Instruction account index mapping
│   │       │   ├── config_action.rs    # Borsh deserialization of ConfigAction args
│   │       │   └── types.rs            # SquadsOperation, DetectedOperation, ConfigChange
│   │       ├── webhook/                # Helius webhook payload handling
│   │       │   ├── types.rs            # Raw transaction serde types
│   │       │   └── mod.rs              # Auth validation, payload parsing
│   │       └── slack/                  # Slack notification building
│   │           ├── types.rs            # Block Kit payload types
│   │           └── mod.rs              # Payload construction and serialization
│   ├── worker-cf/                      # Cloudflare Worker binary
│   │   └── src/
│   │       ├── lib.rs                  # Worker entry point and HTTP handler
│   │       └── config.rs              # Config from wrangler vars/secrets
│   └── worker-lambda/                  # AWS Lambda binary
│       └── src/
│           ├── main.rs                 # Lambda handler with reqwest for Slack
│           └── config.rs              # Config from environment variables
```

## Prerequisites

- **Rust** (stable, 1.75+)
- **wasm32-unknown-unknown target** (for Cloudflare Workers):
  ```sh
  rustup target add wasm32-unknown-unknown
  ```
- **Wrangler CLI** (for Cloudflare Workers):
  ```sh
  npm install -g wrangler
  ```
- **cargo-lambda** (for AWS Lambda, optional):
  ```sh
  cargo install cargo-lambda
  ```
- A [Helius](https://helius.dev) account (free tier works for webhooks)
- A [Slack incoming webhook URL](https://api.slack.com/messaging/webhooks)

## Building

### Run Tests

```sh
cargo test --workspace
```

### Cloudflare Worker

```sh
cargo build -p multisig-monitor-cf --target wasm32-unknown-unknown --release
```

Or use Wrangler (handles the WASM build automatically):

```sh
npx wrangler dev    # local development server
npx wrangler deploy # deploy to Cloudflare
```

### AWS Lambda

For native compilation (same architecture as your machine):

```sh
cargo build -p multisig-monitor-lambda --release
```

For cross-compilation to Lambda's Linux runtime, use cargo-lambda:

```sh
cargo lambda build -p multisig-monitor-lambda --release --arm64
```

The binary is named `bootstrap` as required by the Lambda custom runtime.

## Deploying

### Cloudflare Workers

1. Set your secrets:

   ```sh
   npx wrangler secret put HELIUS_AUTH_TOKEN
   npx wrangler secret put SLACK_WEBHOOK_URL
   ```

2. Edit `wrangler.toml` to configure your multisig addresses and optional filters (see [Configuration](#configuration) below).

3. Deploy:

   ```sh
   npx wrangler deploy
   ```

4. Note the deployed URL (e.g., `https://multisig-monitor.<your-subdomain>.workers.dev`).

### AWS Lambda

1. Build for Lambda:

   ```sh
   cargo lambda build -p multisig-monitor-lambda --release --arm64
   ```

2. Deploy (creates/updates the Lambda function):

   ```sh
   cargo lambda deploy multisig-monitor-lambda \
     --env-var MULTISIG_ADDRESSES="Addr1,Addr2" \
     --env-var HELIUS_AUTH_TOKEN="your-token" \
     --env-var SLACK_WEBHOOK_URL="https://hooks.slack.com/services/..." \
     --env-var INSTRUCTION_TYPES="" \
     --env-var CONFIG_TYPES=""
   ```

   Or configure environment variables in the AWS Console / your IaC tool.

3. Create a **Function URL** (or API Gateway) to expose the Lambda over HTTPS.

### Helius Webhook Setup

After deploying either target, configure a Helius raw webhook to push transactions to your endpoint.

**Via the Helius dashboard:**

1. Go to [dashboard.helius.dev/webhooks](https://dashboard.helius.dev/webhooks).
2. Click **New Webhook**.
3. Set **Webhook Type** to **Raw**.
4. Set **Webhook URL** to `https://<your-worker-or-lambda-url>/webhook`.
5. Add your multisig addresses to the **Account Addresses** list.
6. Set **Auth Header** to the same value you configured for `HELIUS_AUTH_TOKEN`.
7. Save.

**Via the Helius API:**

```sh
curl -X POST "https://api.helius.xyz/v0/webhooks?api-key=<YOUR_API_KEY>" \
  -H "Content-Type: application/json" \
  -d '{
    "webhookURL": "https://<your-endpoint>/webhook",
    "transactionTypes": ["ANY"],
    "accountAddresses": ["<multisig-address-1>", "<multisig-address-2>"],
    "webhookType": "raw",
    "authHeader": "<your-HELIUS_AUTH_TOKEN-value>"
  }'
```

## Configuration

All configuration is provided through environment variables. For Cloudflare Workers, non-secret values go in `wrangler.toml` under `[vars]` and secrets are set via `wrangler secret put`. For AWS Lambda, all values are standard environment variables.

### Required

| Variable | Description |
|---|---|
| `MULTISIG_ADDRESSES` | Comma-separated list of Squads v4 multisig addresses to monitor. |
| `HELIUS_AUTH_TOKEN` | Shared secret that Helius sends in the `Authorization` header of each webhook POST. Used to verify requests are from Helius. |
| `SLACK_WEBHOOK_URL` | Slack incoming webhook URL where notifications are sent. |

**Example:**

```sh
MULTISIG_ADDRESSES="7nYBkp3kFd9Gf2jK8mNpQr5tUvWx,9xKzmR2pLn4sHj7wBcDfAe8qYt6v"
HELIUS_AUTH_TOKEN="whsec_a1b2c3d4e5f6"
SLACK_WEBHOOK_URL="https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXXXXXX"
```

### Optional Filters

| Variable | Default | Description |
|---|---|---|
| `INSTRUCTION_TYPES` | `""` (all) | Comma-separated list of Squads instruction types to notify on. If empty or omitted, all instruction types generate notifications. |
| `CONFIG_TYPES` | `""` (all) | Comma-separated list of config action types to include in `config_transaction_create` notifications. If empty or omitted, all config action types are included. **Ignored if `config_transaction_create` is not in `INSTRUCTION_TYPES` (when `INSTRUCTION_TYPES` is explicitly set).** |

**Example — monitor only proposal votes and config changes:**

```sh
INSTRUCTION_TYPES="proposal_approve,proposal_reject,config_transaction_create"
CONFIG_TYPES="add_member,remove_member,change_threshold"
```

**Example — monitor everything (default):**

```sh
INSTRUCTION_TYPES=""
CONFIG_TYPES=""
```

**Example — monitor only proposal approvals, nothing else:**

```sh
INSTRUCTION_TYPES="proposal_approve"
```

**Example — monitor all instruction types but only alert on member changes in config transactions:**

```sh
INSTRUCTION_TYPES=""
CONFIG_TYPES="add_member,remove_member"
```

### Filter Behavior

- When `INSTRUCTION_TYPES` is **empty**: all 9 instruction types produce notifications.
- When `INSTRUCTION_TYPES` is **set**: only listed types produce notifications. Any instruction not in the list is silently dropped.
- When `CONFIG_TYPES` is **empty**: all 7 config action types are included in `config_transaction_create` notifications.
- When `CONFIG_TYPES` is **set**: only listed config action types are included. If a `config_transaction_create` instruction contains no matching actions after filtering, the notification is suppressed entirely.
- When `INSTRUCTION_TYPES` is set and **does not include** `config_transaction_create`: the `CONFIG_TYPES` setting is ignored (config transactions are not processed at all).

Invalid values in either list cause a startup error with a message listing all valid values.

## Instruction Types Glossary

These are the valid values for `INSTRUCTION_TYPES`. Each maps to a Squads v4 Anchor instruction. The monitor covers **all 32 non-program-config instructions** in the Squads v4 program.

### Multisig Creation

| Value | Description |
|---|---|
| `multisig_create` | Deprecated multisig creation instruction. |
| `multisig_create_v2` | Creates a new multisig. Notification includes the **creator**, initial **members** with permissions, **threshold**, and **config authority** (if set). |

### Proposal Lifecycle

| Value | Description |
|---|---|
| `proposal_create` | A member creates a new proposal for a pending transaction. Notification includes the **creator** and **proposal address**. |
| `proposal_activate` | A member activates a draft proposal, making it eligible for voting. |
| `proposal_approve` | A member with Voter permission casts an approval vote. |
| `proposal_reject` | A member with Voter permission casts a rejection vote. |
| `proposal_cancel` | A member cancels an approved proposal before execution. |
| `proposal_cancel_v2` | V2 variant of proposal cancellation with updated account handling. |

### Transaction Creation

| Value | Description |
|---|---|
| `vault_transaction_create` | Creates a vault transaction (arbitrary instructions to execute from the multisig vault). Notification includes the **transaction address** for correlation. |
| `config_transaction_create` | Creates a config transaction that modifies multisig settings. Notification includes **config actions** (see [Config Types Glossary](#config-types-glossary)) and **transaction address**. |
| `vault_transaction_create_from_buffer` | Creates a vault transaction from a previously uploaded buffer. |

### Transaction Execution

| Value | Description |
|---|---|
| `vault_transaction_execute` | Executes an approved vault transaction. Notification includes the **executor**, **transaction address**, and **proposal address** for correlation back to the originating create and proposal. |
| `config_transaction_execute` | Executes an approved config transaction, applying config changes. Notification includes **transaction address** and **proposal address**. |

### Direct Config Changes (Controlled Multisigs)

These instructions bypass the proposal flow entirely. They are only available on multisigs with a non-zero `config_authority` — the authority can unilaterally modify the multisig without member voting. Notifications for these operations are prefixed with **"Direct:"** and show the **Config Authority** as the acting party.

| Value | Description |
|---|---|
| `multisig_add_member` | Immediately adds a member. Notification includes the **new member address** and **permissions**. |
| `multisig_remove_member` | Immediately removes a member. Notification includes the **removed member address**. |
| `multisig_change_threshold` | Immediately changes the approval threshold. Notification includes the **new threshold**. |
| `multisig_set_time_lock` | Immediately sets the time lock duration. Notification includes the **duration in seconds**. |
| `multisig_set_config_authority` | Transfers config authority to a new address. Notification includes the **new authority address**. |
| `multisig_set_rent_collector` | Sets or removes the rent collector. Notification includes the **new collector address** or indicates removal. |
| `multisig_add_spending_limit` | Creates a spending limit. Notification includes **amount**, **mint**, and **vault index**. |
| `multisig_remove_spending_limit` | Removes a spending limit. |

### Spending Limits

| Value | Description |
|---|---|
| `spending_limit_use` | A member uses a spending limit to transfer tokens without a proposal. Notification includes the **amount** and **decimals**. |

### Buffer Operations

| Value | Description |
|---|---|
| `transaction_buffer_create` | Creates a transaction buffer for large transactions that exceed a single instruction. |
| `transaction_buffer_extend` | Appends data to an existing transaction buffer. |
| `transaction_buffer_close` | Closes (deallocates) a transaction buffer. |

### Batch Operations

| Value | Description |
|---|---|
| `batch_create` | Creates a batch for grouping multiple vault transactions. |
| `batch_add_transaction` | Adds a transaction to an existing batch. Notification includes **transaction** and **proposal addresses**. |
| `batch_execute_transaction` | Executes a transaction within a batch. Notification includes **transaction** and **proposal addresses**. |

### Account Cleanup

| Value | Description |
|---|---|
| `config_transaction_accounts_close` | Closes config transaction accounts and reclaims rent. |
| `vault_transaction_accounts_close` | Closes vault transaction accounts and reclaims rent. |
| `vault_batch_transaction_account_close` | Closes a vault batch transaction account. |
| `batch_accounts_close` | Closes batch accounts and reclaims rent. |

## Config Types Glossary

These are the valid values for `CONFIG_TYPES`. Each maps to a variant of the `ConfigAction` enum in the Squads v4 program, passed as arguments to the `config_transaction_create` instruction. A single config transaction can contain multiple actions.

| Value | Config Action | Description |
|---|---|---|
| `add_member` | `AddMember` | Adds a new member to the multisig. The notification includes the new member's address and their permission set (any combination of **Initiate**, **Vote**, **Execute**). |
| `remove_member` | `RemoveMember` | Removes an existing member from the multisig. The notification includes the removed member's address. |
| `change_threshold` | `ChangeThreshold` | Changes the approval threshold — the number of member votes required to approve a proposal. The notification includes the **new threshold** value. |
| `set_time_lock` | `SetTimeLock` | Sets or updates the time lock period (in seconds) that must elapse between proposal approval and execution. The notification includes the **duration in seconds**. |
| `add_spending_limit` | `AddSpendingLimit` | Creates a spending limit allowing designated members to transfer tokens from a vault without a full proposal/vote cycle. The notification includes the **token mint**, **amount**, and **vault index**. |
| `remove_spending_limit` | `RemoveSpendingLimit` | Removes an existing spending limit. The notification includes the spending limit account address. |
| `set_rent_collector` | `SetRentCollector` | Sets or removes the rent collector address for the multisig. The notification includes the **new collector address**, or indicates removal if set to none. |

## Notification Format

Notifications are sent as Slack [Block Kit](https://api.slack.com/block-kit) messages. All public key addresses and transaction signatures are shown in full (not truncated) for easy copy-paste and auditability. Each notification includes:

- **Operation name** with an emoji indicator
- **Multisig address** — full public key
- **Member address** and their role (Creator, Approver, Rejector, Executor, Config Authority, etc.)
- **Transaction address** and **Proposal address** when available, for correlating creates, votes, and executions
- **Transaction signature** as a clickable link to [Solscan](https://solscan.io)
- **Operation-specific details**: config actions, member permissions, threshold values, spending limit parameters, etc.

### Proposal Approval

```
✅ Proposal Approved

Multisig: SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc
Approver: 9xKzmR2pLn4sHj7wBcDfAe8qYt6vXkZ3nPo1uWr5mQjS

Proposal: Prop1ABCDEFGHijk234567890abcdefghijklmnopqrstuv

Tx: 5nNtjezQMYBHvgSQmoRmJPiXGsPAWmJPoGSa64xanqrauogiVz (link to Solscan)
```

### Config Transaction Created (Proposal-Based)

```
⚙️ Config Transaction Created

Multisig: SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc
Creator: 9xKzmR2pLn4sHj7wBcDfAe8qYt6vXkZ3nPo1uWr5mQjS

Transaction: TxAddr1234567890abcdefghijklmnopqrstuvwxyz12

Config Actions:
  • Add member HjK4NewMember567890abcdefghijklmnopqrstuv12 (permissions: Vote, Execute)
  • Change threshold to 3

Tx: 5nNtjezQMYBHvgSQmoRmJPiXGsPAWmJPoGSa64xanqrauogiVz (link to Solscan)
```

### Vault Transaction Executed

```
🚀 Vault Transaction Executed

Multisig: SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc
Executor: 9xKzmR2pLn4sHj7wBcDfAe8qYt6vXkZ3nPo1uWr5mQjS

Transaction: TxAddr1234567890abcdefghijklmnopqrstuvwxyz12
Proposal: PropAddr567890abcdefghijklmnopqrstuvwxyz1234

Tx: 5nNtjezQMYBHvgSQmoRmJPiXGsPAWmJPoGSa64xanqrauogiVz (link to Solscan)
```

### Direct Member Added (Controlled Multisig)

Direct config changes bypass the proposal flow and are flagged with a warning indicator.

```
⚠️ Direct: Member Added

Multisig: SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc
Config Authority: AuthKeyABCDEF1234567890abcdefghijklmnopqrstuv

New Member: HjK4NewMember567890abcdefghijklmnopqrstuv12
Permissions: Initiate, Vote, Execute

Tx: 5nNtjezQMYBHvgSQmoRmJPiXGsPAWmJPoGSa64xanqrauogiVz (link to Solscan)
```

### Direct Threshold Changed (Controlled Multisig)

```
⚠️ Direct: Threshold Changed

Multisig: SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc
Config Authority: AuthKeyABCDEF1234567890abcdefghijklmnopqrstuv

New Threshold: 3

Tx: 5nNtjezQMYBHvgSQmoRmJPiXGsPAWmJPoGSa64xanqrauogiVz (link to Solscan)
```

### Spending Limit Used

```
💸 Spending Limit Used

Multisig: SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc
Member: 9xKzmR2pLn4sHj7wBcDfAe8qYt6vXkZ3nPo1uWr5mQjS

Amount: 1000000 (decimals: 6)

Tx: 5nNtjezQMYBHvgSQmoRmJPiXGsPAWmJPoGSa64xanqrauogiVz (link to Solscan)
```

## Endpoints

Both deployment targets expose the same HTTP interface:

| Method | Path | Description |
|---|---|---|
| `POST` | `/webhook` | Receives Helius raw webhook payloads. Validates the `Authorization` header, parses transactions, and sends Slack notifications. Returns `200` on success. |
| `GET` | `/health` | Returns `200 ok`. Use for uptime monitoring. |

## Squads v4 Program

This monitor targets the Squads Multisig Program v4:

- **Program ID:** `SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf`
- **Network:** Solana Mainnet
- **Framework:** Anchor (instructions identified by 8-byte SHA-256 discriminators)

### Why the Squads types are duplicated

The `multisig-monitor-core` crate contains its own Borsh-compatible definitions for Squads v4 types (`ConfigAction`, `Member`, `Permissions`, instruction arg structs, etc.) rather than importing them from the official [`squads-multisig`](https://crates.io/crates/squads-multisig) SDK crate.

This is intentional. The `squads-multisig` crate depends on `squads-multisig-program`, which transitively pulls in `anchor-lang`, `solana-program`, and `solana-rpc-client`. These crates assume access to the Solana runtime (BPF syscalls, OS-level networking, full `std`) and **do not compile for `wasm32-unknown-unknown`** — the target required by Cloudflare Workers and other constrained WASM runtimes. There are no feature flags in the SDK to opt out of these dependencies.

The duplicated types in `crates/core/src/squads/config_action.rs` are ~100 lines of struct and enum definitions with `#[derive(BorshDeserialize)]`, depending only on the `borsh` and `bs58` crates (both pure Rust, WASM-safe). They mirror the on-chain layout exactly as defined in the [Squads v4 IDL](https://github.com/Squads-Protocol/v4/blob/main/sdk/multisig/idl/squads_multisig_program.json), and are verified by unit tests that round-trip through Borsh serialization. If the Squads program updates its type layouts in a future version, these definitions will need to be updated to match.

## Development

```sh
# Run all tests
cargo test --workspace

# Lint
cargo clippy --workspace

# Local CF Worker development server
npx wrangler dev

# Test the webhook endpoint locally
curl -X POST http://localhost:8787/webhook \
  -H "Authorization: your-token" \
  -H "Content-Type: application/json" \
  -d '[{"signature":"test","slot":0,"transaction":{"message":{"accountKeys":[],"instructions":[]},"signatures":[]},"meta":null}]'
```

## License

MIT

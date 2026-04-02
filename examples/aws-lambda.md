# Deploying Multisig Monitor on AWS Lambda

This guide walks through deploying the multisig monitor as an AWS Lambda function, configured to monitor **all config transactions** for a single Squads v4 multisig.

## Example Values

Throughout this guide we use these placeholder values:

| Value | Example |
|---|---|
| Multisig address | `SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc` |
| Helius API key | `aaa111-bb22-cc33-dd44-eee555fff666` |
| Webhook auth token | `whsec_m0n1t0r_s3cr3t_t0k3n` |
| Slack webhook URL | `https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere` |
| AWS region | `us-east-1` |

Replace these with your actual values.

## Prerequisites

- An [AWS account](https://aws.amazon.com/) with permissions to create Lambda functions
- [AWS CLI](https://aws.amazon.com/cli/) configured (`aws configure`)
- [Rust](https://rustup.rs/) (stable 1.75+)
- [cargo-lambda](https://www.cargo-lambda.info/):
  ```sh
  cargo install cargo-lambda
  ```
- A [Helius account](https://dashboard.helius.dev/) (free tier supports webhooks)
- A [Slack incoming webhook](https://api.slack.com/messaging/webhooks)

## Step 1: Build the Lambda Binary

From the project root, build for the Lambda ARM64 runtime (cheaper and faster than x86):

```sh
cargo lambda build -p multisig-monitor-lambda --release --arm64
```

This cross-compiles the binary and produces `target/lambda/multisig-monitor-lambda/bootstrap`.

For x86 Lambda environments, omit `--arm64`:

```sh
cargo lambda build -p multisig-monitor-lambda --release
```

## Step 2: Deploy the Lambda Function

### Option A: Using cargo-lambda (Recommended)

Deploy with environment variables inline:

```sh
cargo lambda deploy multisig-monitor-lambda \
  --region us-east-1 \
  --env-var MULTISIG_ADDRESSES="SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc" \
  --env-var INSTRUCTION_TYPES="config_transaction_create,config_transaction_execute,multisig_add_member,multisig_remove_member,multisig_change_threshold,multisig_set_time_lock,multisig_set_config_authority,multisig_set_rent_collector,multisig_add_spending_limit,multisig_remove_spending_limit" \
  --env-var CONFIG_TYPES="" \
  --env-var HELIUS_AUTH_TOKEN="whsec_m0n1t0r_s3cr3t_t0k3n" \
  --env-var SLACK_WEBHOOK_URL="https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere"
```

This creates the Lambda function, uploads the binary, and sets the environment variables.

### Option B: Using AWS CLI

1. Create a deployment package:

   ```sh
   cd target/lambda/multisig-monitor-lambda
   zip -j bootstrap.zip bootstrap
   cd -
   ```

2. Create an IAM execution role (if you don't have one):

   ```sh
   aws iam create-role \
     --role-name multisig-monitor-role \
     --assume-role-policy-document '{
       "Version": "2012-10-17",
       "Statement": [{
         "Effect": "Allow",
         "Principal": {"Service": "lambda.amazonaws.com"},
         "Action": "sts:AssumeRole"
       }]
     }'

   aws iam attach-role-policy \
     --role-name multisig-monitor-role \
     --policy-arn arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole
   ```

3. Create the Lambda function:

   ```sh
   aws lambda create-function \
     --function-name multisig-monitor \
     --runtime provided.al2023 \
     --architectures arm64 \
     --handler bootstrap \
     --role arn:aws:iam::<ACCOUNT_ID>:role/multisig-monitor-role \
     --zip-file fileb://target/lambda/multisig-monitor-lambda/bootstrap.zip \
     --timeout 30 \
     --memory-size 128 \
     --environment '{
       "Variables": {
         "MULTISIG_ADDRESSES": "SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc",
         "INSTRUCTION_TYPES": "config_transaction_create,config_transaction_execute,multisig_add_member,multisig_remove_member,multisig_change_threshold,multisig_set_time_lock,multisig_set_config_authority,multisig_set_rent_collector,multisig_add_spending_limit,multisig_remove_spending_limit",
         "CONFIG_TYPES": "",
         "HELIUS_AUTH_TOKEN": "whsec_m0n1t0r_s3cr3t_t0k3n",
         "SLACK_WEBHOOK_URL": "https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere"
       }
     }'
   ```

## Step 3: Create a Function URL

The Lambda needs a public HTTPS endpoint for Helius to POST to. A Function URL is the simplest way:

```sh
aws lambda create-function-url-config \
  --function-name multisig-monitor \
  --auth-type NONE
```

This returns a URL like:

```json
{
  "FunctionUrl": "https://abc123xyz.lambda-url.us-east-1.on.aws/"
}
```

Grant public access (the Worker validates auth via the `Authorization` header, not Lambda auth):

```sh
aws lambda add-permission \
  --function-name multisig-monitor \
  --statement-id public-access \
  --action lambda:InvokeFunctionUrl \
  --principal "*" \
  --function-url-auth-type NONE
```

Note the Function URL — you need it for the Helius webhook.

## Step 4: Verify the Deployment

```sh
curl https://abc123xyz.lambda-url.us-east-1.on.aws/health
```

Expected response: `ok`

Test that unauthorized requests are rejected:

```sh
curl -s -o /dev/null -w "%{http_code}" \
  -X POST https://abc123xyz.lambda-url.us-east-1.on.aws/webhook \
  -H "Authorization: wrong-token" \
  -d '[]'
```

Expected response: `401`

## Step 5: Configure the Helius Webhook

### Option A: Via the Helius Dashboard

1. Go to [dashboard.helius.dev/webhooks](https://dashboard.helius.dev/webhooks)
2. Click **New Webhook**
3. Fill in:
   - **Webhook Type**: `Raw`
   - **Webhook URL**: `https://abc123xyz.lambda-url.us-east-1.on.aws/webhook`
   - **Account Addresses**: `SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc`
   - **Transaction Types**: `ANY` (the Lambda handles filtering)
   - **Auth Header**: `whsec_m0n1t0r_s3cr3t_t0k3n`
4. Click **Create Webhook**

### Option B: Via the Helius API

```sh
curl -X POST "https://api.helius.xyz/v0/webhooks?api-key=aaa111-bb22-cc33-dd44-eee555fff666" \
  -H "Content-Type: application/json" \
  -d '{
    "webhookURL": "https://abc123xyz.lambda-url.us-east-1.on.aws/webhook",
    "transactionTypes": ["ANY"],
    "accountAddresses": ["SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc"],
    "webhookType": "raw",
    "authHeader": "whsec_m0n1t0r_s3cr3t_t0k3n"
  }'
```

Save the returned webhook ID for later management.

## Step 6: Test End-to-End

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

## Sensitive Values: Using AWS Secrets Manager (Optional)

The basic setup stores `HELIUS_AUTH_TOKEN` and `SLACK_WEBHOOK_URL` as plain environment variables. For production, you may want to use AWS Secrets Manager or SSM Parameter Store instead. The Lambda function reads these from `std::env::var`, so you would need to add a secrets-fetching layer or use a Lambda extension.

A simpler alternative is to encrypt the environment variables at rest with a KMS key:

```sh
aws lambda update-function-configuration \
  --function-name multisig-monitor \
  --kms-key-arn arn:aws:kms:us-east-1:<ACCOUNT_ID>:key/<KEY_ID>
```

## Local Development

Test the Lambda locally with cargo-lambda's built-in emulator:

```sh
MULTISIG_ADDRESSES="SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc" \
INSTRUCTION_TYPES="config_transaction_create,config_transaction_execute,multisig_add_member,multisig_remove_member,multisig_change_threshold,multisig_set_time_lock,multisig_set_config_authority,multisig_set_rent_collector,multisig_add_spending_limit,multisig_remove_spending_limit" \
CONFIG_TYPES="" \
HELIUS_AUTH_TOKEN="whsec_m0n1t0r_s3cr3t_t0k3n" \
SLACK_WEBHOOK_URL="https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere" \
cargo lambda watch -p multisig-monitor-lambda
```

This starts a local server at `http://localhost:9000`. Test with:

```sh
curl -X POST http://localhost:9000/webhook \
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

## Updating Configuration

To update environment variables:

```sh
aws lambda update-function-configuration \
  --function-name multisig-monitor \
  --environment '{
    "Variables": {
      "MULTISIG_ADDRESSES": "SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc",
      "INSTRUCTION_TYPES": "config_transaction_create,config_transaction_execute,multisig_add_member,multisig_remove_member,multisig_change_threshold,multisig_set_time_lock,multisig_set_config_authority,multisig_set_rent_collector,multisig_add_spending_limit,multisig_remove_spending_limit",
      "CONFIG_TYPES": "",
      "HELIUS_AUTH_TOKEN": "whsec_m0n1t0r_s3cr3t_t0k3n",
      "SLACK_WEBHOOK_URL": "https://hooks.slack.com/services/T024F6G8L/B07EXAMPLE/xyzExampleTokenHere"
    }
  }'
```

Note: `update-function-configuration` replaces **all** environment variables. Always include the full set.

To deploy new code after changes:

```sh
cargo lambda build -p multisig-monitor-lambda --release --arm64
cargo lambda deploy multisig-monitor-lambda --region us-east-1
```

## Monitoring and Logs

View Lambda logs in CloudWatch:

```sh
aws logs tail /aws/lambda/multisig-monitor --follow
```

Or use cargo-lambda:

```sh
cargo lambda logs --function-name multisig-monitor --follow
```

The Lambda emits structured logs via `tracing`. Set the `RUST_LOG` environment variable to control verbosity:

```sh
aws lambda update-function-configuration \
  --function-name multisig-monitor \
  --environment '{
    "Variables": {
      "RUST_LOG": "info",
      ...
    }
  }'
```

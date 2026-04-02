mod config;

use config::Config;
use lambda_http::{Body, Request, Response, run, service_fn};
use multisig_monitor_core::{slack, squads, webhook};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time()
        .init();

    run(service_fn(handler)).await
}

async fn handler(req: Request) -> Result<Response<Body>, lambda_runtime::Error> {
    let config = Config::from_env()?;

    let (parts, body) = req.into_parts();

    // Route based on path + method
    let path = parts.uri.path();
    let method = parts.method.as_str();

    match (method, path) {
        ("GET", "/health") => Ok(Response::builder()
            .status(200)
            .body(Body::Text("ok".to_string()))?),

        ("POST", "/webhook") => handle_webhook(&parts.headers, body, &config).await,

        _ => Ok(Response::builder()
            .status(404)
            .body(Body::Text("not found".to_string()))?),
    }
}

async fn handle_webhook(
    headers: &lambda_http::http::HeaderMap,
    body: Body,
    config: &Config,
) -> Result<Response<Body>, lambda_runtime::Error> {
    // Validate auth
    let auth_header = headers.get("Authorization").and_then(|v| v.to_str().ok());

    if let Err(e) = webhook::validate_auth(auth_header, &config.helius_auth_token) {
        tracing::warn!("auth failed: {e}");
        return Ok(Response::builder()
            .status(401)
            .body(Body::Text("unauthorized".to_string()))?);
    }

    // Parse body
    let body_str = match body {
        Body::Text(s) => s,
        Body::Binary(b) => String::from_utf8(b)?,
        Body::Empty => String::new(),
    };

    let transactions = match webhook::parse_payload(&body_str) {
        Ok(txs) => txs,
        Err(e) => {
            tracing::error!("payload parse failed: {e}");
            return Ok(Response::builder()
                .status(400)
                .body(Body::Text(format!("bad request: {e}")))?);
        }
    };

    let operations =
        squads::extract_operations(&transactions, &config.multisig_addresses, &config.filter);

    let client = reqwest::Client::new();
    for op in &operations {
        if let Err(e) = send_slack(&client, op, &config.slack_webhook_url).await {
            tracing::error!("slack notification failed: {e}");
        }
    }

    Ok(Response::builder()
        .status(200)
        .body(Body::Text("ok".to_string()))?)
}

async fn send_slack(
    client: &reqwest::Client,
    op: &squads::types::DetectedOperation,
    webhook_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let body = slack::serialize_payload(op)?;

    let resp = client
        .post(webhook_url)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await?;

    let status = resp.status().as_u16();
    if status != 200 {
        return Err(format!("slack returned status {status}").into());
    }

    Ok(())
}

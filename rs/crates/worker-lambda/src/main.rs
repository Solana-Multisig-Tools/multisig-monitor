mod config;

use config::Config;
use lambda_http::{Body, Request, Response, run, service_fn};
use multisig_monitor_core::log::{LogLevel, log_json};
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
        if let Some(json) = log_json(LogLevel::Error, config.log_level, "auth failed", Some(serde_json::json!({ "error": e.to_string() }))) {
            eprintln!("{json}");
        }
        return Ok(Response::builder()
            .status(401)
            .body(Body::Text("unauthorized".to_string()))?);
    }

    // Parse body
    const MAX_BODY_SIZE: usize = 5 * 1024 * 1024; // 5 MB
    let body_str = match body {
        Body::Text(s) => s,
        Body::Binary(b) => String::from_utf8(b)?,
        Body::Empty => String::new(),
    };
    if body_str.len() > MAX_BODY_SIZE {
        return Ok(Response::builder()
            .status(413)
            .body(Body::Text("payload too large".to_string()))?);
    }

    let transactions = match webhook::parse_payload(&body_str) {
        Ok(txs) => txs,
        Err(e) => {
            if let Some(json) = log_json(LogLevel::Error, config.log_level, "payload parse failed", Some(serde_json::json!({ "error": e.to_string() }))) {
                eprintln!("{json}");
            }
            return Ok(Response::builder()
                .status(400)
                .body(Body::Text(format!("bad request: {e}")))?);
        }
    };

    let operations =
        squads::extract_operations(&transactions, &config.multisig_addresses, &config.filter);

    if let Some(json) = log_json(LogLevel::Info, config.log_level, "webhook processed", Some(serde_json::json!({
        "transactions": transactions.len(),
        "operations": operations.len(),
    }))) {
        println!("{json}");
    }

    let client = reqwest::Client::new();
    let slack_futures = operations
        .iter()
        .map(|op| send_slack(&client, op, &config.slack_webhook_url, &config.display));
    let results = futures::future::join_all(slack_futures).await;
    for result in results {
        if let Err(e) = result
            && let Some(json) = log_json(LogLevel::Error, config.log_level, "slack notification failed", Some(serde_json::json!({ "error": e.to_string() })))
        {
            eprintln!("{json}");
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
    display: &multisig_monitor_core::log::DisplayConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let body = slack::serialize_payload(op, display)?;

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

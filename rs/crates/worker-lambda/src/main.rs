mod config;

use std::collections::{HashMap, HashSet};

use futures::future;
use lambda_http::{Body, Request, Response, run, service_fn};
use multisig_monitor_core::{
    log::{DisplayConfig, LogLevel, log_json},
    slack, squads, webhook,
};
use tokio::time::{Duration, sleep};
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
    let (parts, body) = req.into_parts();

    // Route based on path + method
    let path = parts.uri.path();
    let method = parts.method.as_str();

    match (method, path) {
        ("GET", "/health") => match config::from_env() {
            Ok(_) => Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::Text(r#"{"status":"ok"}"#.to_string()))?),
            Err(e) => Ok(Response::builder()
                .status(500)
                .header("Content-Type", "application/json")
                .body(Body::Text(
                    serde_json::json!({ "status": "error", "message": e.to_string() }).to_string(),
                ))?),
        },

        ("POST", "/webhook") => {
            let config = config::from_env()?;
            handle_webhook(&parts.headers, body, &config).await
        }

        _ => Ok(Response::builder()
            .status(404)
            .body(Body::Text("not found".to_string()))?),
    }
}

async fn handle_webhook(
    headers: &lambda_http::http::HeaderMap,
    body: Body,
    config: &config::Config,
) -> Result<Response<Body>, lambda_runtime::Error> {
    // Validate auth
    let auth_header = headers.get("Authorization").and_then(|v| v.to_str().ok());

    if let Err(e) = webhook::validate_auth(auth_header, &config.helius_auth_token) {
        if let Some(json) = log_json(
            LogLevel::Error,
            config.log_level,
            "auth failed",
            Some(serde_json::json!({ "error": e.to_string() })),
        ) {
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
        other => String::from_utf8(other.to_vec())?,
    };
    if body_str.len() > MAX_BODY_SIZE {
        return Ok(Response::builder()
            .status(413)
            .body(Body::Text("payload too large".to_string()))?);
    }

    let transactions = match webhook::parse_payload(&body_str) {
        Ok(txs) => txs,
        Err(e) => {
            if let Some(json) = log_json(
                LogLevel::Error,
                config.log_level,
                "payload parse failed",
                Some(serde_json::json!({ "error": e.to_string() })),
            ) {
                eprintln!("{json}");
            }
            return Ok(Response::builder()
                .status(400)
                .body(Body::Text(format!("bad request: {e}")))?);
        }
    };

    let operations =
        squads::extract_operations(&transactions, &config.multisig_addresses, &config.filter);

    // Dedup by signature
    let mut seen = HashSet::new();
    let deduped: Vec<_> = operations
        .into_iter()
        .filter(|op| seen.insert(op.signature.clone()))
        .collect();

    if let Some(json) = log_json(
        LogLevel::Info,
        config.log_level,
        "webhook processed",
        Some(serde_json::json!({
            "transactions": transactions.len(),
            "operations": deduped.len(),
        })),
    ) {
        println!("{json}");
    }

    // Group by destination webhook URL
    let mut groups: HashMap<String, Vec<&squads::types::DetectedOperation>> = HashMap::new();
    for op in &deduped {
        let url = config.resolve_webhook_url(&op.multisig).to_string();
        groups.entry(url).or_default().push(op);
    }

    let client = reqwest::Client::new();
    let send_futures = groups.into_iter().map(|(url, ops)| {
        let client = &client;
        let display = &config.display;
        async move { send_batch_slack(client, &ops, &url, display).await }
    });
    let results = future::join_all(send_futures).await;
    for result in results {
        if let Err(e) = result
            && let Some(json) = log_json(
                LogLevel::Error,
                config.log_level,
                "slack notification failed",
                Some(serde_json::json!({ "error": e.to_string() })),
            )
        {
            eprintln!("{json}");
        }
    }

    Ok(Response::builder()
        .status(200)
        .body(Body::Text("ok".to_string()))?)
}

async fn send_batch_slack(
    client: &reqwest::Client,
    ops: &[&squads::types::DetectedOperation],
    webhook_url: &str,
    display: &DisplayConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let owned: Vec<_> = ops.iter().copied().cloned().collect();
    let bodies = slack::serialize_batch_payloads(&owned, display)?;

    for body in bodies {
        let resp = client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .body(body.clone())
            .send()
            .await?;

        let status = resp.status().as_u16();
        if status == 200 {
            continue;
        }

        if status == 429 || status >= 500 {
            sleep(Duration::from_secs(1)).await;
            let retry = client
                .post(webhook_url)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await?;
            let retry_status = retry.status().as_u16();
            if retry_status != 200 {
                return Err(format!("slack returned status {retry_status} after retry").into());
            }
            continue;
        }

        return Err(format!("slack returned status {status}").into());
    }
    Ok(())
}

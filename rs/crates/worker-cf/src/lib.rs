mod config;

use config::Config;
use multisig_monitor_core::error::MonitorError;
use multisig_monitor_core::log::{LogLevel, log_json};
use multisig_monitor_core::{slack, squads, webhook};
use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    Router::new()
        .post_async("/webhook", handle_webhook)
        .get_async("/health", |_, ctx| async move {
            match Config::from_env(&ctx.env) {
                Ok(_) => Response::from_json(&serde_json::json!({ "status": "ok" })),
                Err(e) => {
                    let body =
                        serde_json::json!({ "status": "error", "message": e.to_string() });
                    Ok(Response::from_json(&body)?.with_status(500))
                }
            }
        })
        .run(req, env)
        .await
}

async fn handle_webhook(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let config = Config::from_env(&ctx.env).map_err(to_worker_err)?;

    let auth_header = req.headers().get("Authorization").ok().flatten();
    if webhook::validate_auth(auth_header.as_deref(), &config.helius_auth_token).is_err() {
        return Response::error("unauthorized", 401);
    }

    const MAX_BODY_SIZE: usize = 5 * 1024 * 1024; // 5 MB
    if let Some(cl) = req.headers().get("content-length").ok().flatten()
        && cl.parse::<usize>().unwrap_or(0) > MAX_BODY_SIZE
    {
        return Response::error("payload too large", 413);
    }

    let body = req.text().await?;
    if body.len() > MAX_BODY_SIZE {
        return Response::error("payload too large", 413);
    }
    let transactions = webhook::parse_payload(&body).map_err(to_worker_err)?;
    let operations =
        squads::extract_operations(&transactions, &config.multisig_addresses, &config.filter);

    if let Some(json) = log_json(LogLevel::Info, config.log_level, "webhook processed", Some(serde_json::json!({
        "transactions": transactions.len(),
        "operations": operations.len(),
    }))) {
        console_log!("{json}");
    }

    let slack_futures = operations
        .iter()
        .map(|op| send_slack(op, &config.slack_webhook_url, &config.display));
    let results = futures::future::join_all(slack_futures).await;
    for result in results {
        if let Err(e) = result
            && let Some(json) = log_json(LogLevel::Error, config.log_level, "slack notification failed", Some(serde_json::json!({ "error": e.to_string() })))
        {
            console_error!("{json}");
        }
    }

    Response::ok("ok")
}

async fn send_slack(
    op: &squads::types::DetectedOperation,
    webhook_url: &str,
    display: &multisig_monitor_core::log::DisplayConfig,
) -> std::result::Result<(), MonitorError> {
    let body = slack::serialize_payload(op, display)?;

    // First attempt
    let status = {
        let headers = Headers::new();
        headers
            .set("Content-Type", "application/json")
            .map_err(|e| MonitorError::Platform(e.to_string()))?;
        let req = Request::new_with_init(
            webhook_url,
            RequestInit::new()
                .with_method(Method::Post)
                .with_headers(headers)
                .with_body(Some(wasm_bindgen::JsValue::from_str(&body))),
        )
        .map_err(|e| MonitorError::Platform(e.to_string()))?;
        Fetch::Request(req)
            .send()
            .await
            .map_err(|e| MonitorError::Platform(e.to_string()))?
            .status_code()
    };

    if status == 200 {
        return Ok(());
    }

    // Retry once on transient errors (rate limit or server error)
    if status == 429 || status >= 500 {
        let headers = Headers::new();
        headers
            .set("Content-Type", "application/json")
            .map_err(|e| MonitorError::Platform(e.to_string()))?;
        let req = Request::new_with_init(
            webhook_url,
            RequestInit::new()
                .with_method(Method::Post)
                .with_headers(headers)
                .with_body(Some(wasm_bindgen::JsValue::from_str(&body))),
        )
        .map_err(|e| MonitorError::Platform(e.to_string()))?;
        let retry_status = Fetch::Request(req)
            .send()
            .await
            .map_err(|e| MonitorError::Platform(e.to_string()))?
            .status_code();

        if retry_status == 200 {
            return Ok(());
        }
        return Err(MonitorError::SlackSend { status: retry_status });
    }

    Err(MonitorError::SlackSend { status })
}

fn to_worker_err(e: MonitorError) -> Error {
    Error::RustError(e.to_string())
}

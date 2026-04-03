mod config;

use config::Config;
use multisig_monitor_core::error::MonitorError;
use multisig_monitor_core::{slack, squads, webhook};
use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    Router::new()
        .post_async("/webhook", handle_webhook)
        .get("/health", |_, _| Response::ok("ok"))
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

    let slack_futures = operations
        .iter()
        .map(|op| send_slack(op, &config.slack_webhook_url, &config.display));
    let results = futures::future::join_all(slack_futures).await;
    for result in results {
        if let Err(e) = result {
            console_error!("slack notification failed: {e}");
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

    let resp = Fetch::Request(req)
        .send()
        .await
        .map_err(|e| MonitorError::Platform(e.to_string()))?;

    match resp.status_code() {
        200 => Ok(()),
        status => Err(MonitorError::SlackSend { status }),
    }
}

fn to_worker_err(e: MonitorError) -> Error {
    Error::RustError(e.to_string())
}

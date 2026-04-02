mod config;
mod error;
mod slack;
mod squads;
mod webhook;

use config::Config;
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
    let config = Config::from_env(&ctx.env)?;

    webhook::validate_auth(&req, &config.helius_auth_token)?;

    let body = req.text().await?;
    let transactions = webhook::parse_payload(&body)?;

    let operations = squads::extract_operations(&transactions, &config.multisig_addresses);

    for op in &operations {
        if let Err(e) = slack::send_notification(op, &config.slack_webhook_url).await {
            worker::console_error!("slack notification failed: {e}");
        }
    }

    Response::ok("ok")
}

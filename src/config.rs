use crate::error::MonitorError;
use worker::Env;

#[derive(Debug, Clone)]
pub struct Config {
    pub multisig_addresses: Vec<String>,
    pub helius_auth_token: String,
    pub slack_webhook_url: String,
}

impl Config {
    pub fn from_env(env: &Env) -> Result<Self, MonitorError> {
        let addresses_raw = env
            .var("MULTISIG_ADDRESSES")
            .map_err(|e| MonitorError::Worker(format!("MULTISIG_ADDRESSES not set: {e}")))?
            .to_string();

        let multisig_addresses: Vec<String> = addresses_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let helius_auth_token = env
            .secret("HELIUS_AUTH_TOKEN")
            .map_err(|e| MonitorError::Worker(format!("HELIUS_AUTH_TOKEN not set: {e}")))?
            .to_string();

        let slack_webhook_url = env
            .secret("SLACK_WEBHOOK_URL")
            .map_err(|e| MonitorError::Worker(format!("SLACK_WEBHOOK_URL not set: {e}")))?
            .to_string();

        Ok(Config {
            multisig_addresses,
            helius_auth_token,
            slack_webhook_url,
        })
    }
}

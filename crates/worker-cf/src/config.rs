use multisig_monitor_core::error::MonitorError;
use multisig_monitor_core::filter::{parse_comma_list, NotificationFilter};
use worker::Env;

#[derive(Debug, Clone)]
pub struct Config {
    pub multisig_addresses: Vec<String>,
    pub helius_auth_token: String,
    pub slack_webhook_url: String,
    pub filter: NotificationFilter,
}

impl Config {
    pub fn from_env(env: &Env) -> Result<Self, MonitorError> {
        let addresses_raw = env
            .var("MULTISIG_ADDRESSES")
            .map_err(|e| MonitorError::Config(format!("MULTISIG_ADDRESSES: {e}")))?
            .to_string();

        let multisig_addresses = parse_comma_list(&addresses_raw);

        let helius_auth_token = env
            .secret("HELIUS_AUTH_TOKEN")
            .map_err(|e| MonitorError::Config(format!("HELIUS_AUTH_TOKEN: {e}")))?
            .to_string();

        let slack_webhook_url = env
            .secret("SLACK_WEBHOOK_URL")
            .map_err(|e| MonitorError::Config(format!("SLACK_WEBHOOK_URL: {e}")))?
            .to_string();

        let instruction_types = env
            .var("INSTRUCTION_TYPES")
            .map(|v| parse_comma_list(&v.to_string()))
            .unwrap_or_default();

        let config_types = env
            .var("CONFIG_TYPES")
            .map(|v| parse_comma_list(&v.to_string()))
            .unwrap_or_default();

        let filter = NotificationFilter {
            instruction_types,
            config_types,
        };
        filter.validate()?;

        Ok(Config {
            multisig_addresses,
            helius_auth_token,
            slack_webhook_url,
            filter,
        })
    }
}

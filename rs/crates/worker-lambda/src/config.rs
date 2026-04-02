use multisig_monitor_core::error::MonitorError;
use multisig_monitor_core::filter::{NotificationFilter, parse_comma_list};

#[derive(Debug, Clone)]
pub struct Config {
    pub multisig_addresses: Vec<String>,
    pub helius_auth_token: String,
    pub slack_webhook_url: String,
    pub filter: NotificationFilter,
}

impl Config {
    pub fn from_env() -> Result<Self, MonitorError> {
        let addresses_raw = std::env::var("MULTISIG_ADDRESSES")
            .map_err(|_| MonitorError::Config("MULTISIG_ADDRESSES not set".to_string()))?;

        let multisig_addresses = parse_comma_list(&addresses_raw);

        let helius_auth_token = std::env::var("HELIUS_AUTH_TOKEN")
            .map_err(|_| MonitorError::Config("HELIUS_AUTH_TOKEN not set".to_string()))?;

        let slack_webhook_url = std::env::var("SLACK_WEBHOOK_URL")
            .map_err(|_| MonitorError::Config("SLACK_WEBHOOK_URL not set".to_string()))?;

        let instruction_types = std::env::var("INSTRUCTION_TYPES")
            .map(|v| parse_comma_list(&v))
            .unwrap_or_default();

        let config_types = std::env::var("CONFIG_TYPES")
            .map(|v| parse_comma_list(&v))
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

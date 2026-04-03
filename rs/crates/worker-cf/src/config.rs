use std::collections::HashMap;

use multisig_monitor_core::{
    error::MonitorError,
    filter::{NotificationFilter, parse_comma_list},
    log::{DisplayConfig, LogLevel, parse_bool, parse_log_level},
};
use worker::Env;

#[derive(Debug, Clone)]
pub struct Config {
    pub multisig_addresses: Vec<String>,
    pub helius_auth_token: String,
    pub slack_webhook_url: String,
    pub filter: NotificationFilter,
    pub display: DisplayConfig,
    pub log_level: LogLevel,
    pub routes: HashMap<String, String>,
}

impl Config {
    pub fn from_env(env: &Env) -> Result<Self, MonitorError> {
        let addresses_raw = env
            .var("MULTISIG_ADDRESSES")
            .map_err(|e| MonitorError::Config(format!("MULTISIG_ADDRESSES: {e}")))?
            .to_string();

        let multisig_addresses = parse_comma_list(&addresses_raw);
        if multisig_addresses.is_empty() {
            return Err(MonitorError::Config(
                "MULTISIG_ADDRESSES is required".to_string(),
            ));
        }

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

        let display = DisplayConfig {
            show_permissions: env
                .var("SHOW_PERMISSIONS")
                .map(|v| parse_bool(&v.to_string()))
                .unwrap_or(false),
            show_timestamps: env
                .var("SHOW_TIMESTAMPS")
                .map(|v| parse_bool(&v.to_string()))
                .unwrap_or(false),
            truncate_addresses: env
                .var("TRUNCATE_ADDRESSES")
                .map(|v| parse_bool(&v.to_string()))
                .unwrap_or(false),
        };

        let log_level = env
            .var("LOG_LEVEL")
            .map(|v| parse_log_level(&v.to_string()))
            .unwrap_or(LogLevel::Error);

        let routes = env
            .var("MULTISIG_ROUTES")
            .map(|v| parse_routes(&v.to_string()))
            .unwrap_or_default();

        Ok(Config {
            multisig_addresses,
            helius_auth_token,
            slack_webhook_url,
            filter,
            display,
            log_level,
            routes,
        })
    }

    pub fn resolve_webhook_url(&self, multisig: &str) -> &str {
        self.routes
            .get(multisig)
            .map(|s| s.as_str())
            .unwrap_or(&self.slack_webhook_url)
    }
}

fn parse_routes(raw: &str) -> HashMap<String, String> {
    let mut routes = HashMap::new();
    if raw.is_empty() {
        return routes;
    }
    for entry in raw.split(',') {
        if let Some((addr, url)) = entry.split_once(':')
            && !addr.trim().is_empty()
            && !url.trim().is_empty()
        {
            routes.insert(addr.to_string(), url.to_string());
        }
    }
    routes
}

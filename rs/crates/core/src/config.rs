use std::collections::HashMap;

use crate::error::MonitorError;
use crate::filter::{NotificationFilter, parse_comma_list};
use crate::log::{DisplayConfig, LogLevel, parse_bool, parse_log_level};

/// Abstraction over platform-specific environment variable access.
///
/// Cloudflare Workers use `worker::Env::var()` / `secret()`, while Lambda
/// and other native targets use `std::env::var()`. Implementing this trait
/// for each platform lets `Config::from_reader` contain the shared parsing
/// logic exactly once.
pub trait EnvReader {
    fn get_var(&self, name: &str) -> Result<String, MonitorError>;
    fn get_optional(&self, name: &str) -> Option<String>;
}

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
    pub fn from_reader(reader: &impl EnvReader) -> Result<Self, MonitorError> {
        let addresses_raw = reader.get_var("MULTISIG_ADDRESSES")?;
        let multisig_addresses = parse_comma_list(&addresses_raw);
        if multisig_addresses.is_empty() {
            return Err(MonitorError::Config(
                "MULTISIG_ADDRESSES is required".to_string(),
            ));
        }

        let helius_auth_token = reader.get_var("HELIUS_AUTH_TOKEN")?;
        let slack_webhook_url = reader.get_var("SLACK_WEBHOOK_URL")?;

        let instruction_types = reader
            .get_optional("INSTRUCTION_TYPES")
            .map(|v| parse_comma_list(&v))
            .unwrap_or_default();

        let config_types = reader
            .get_optional("CONFIG_TYPES")
            .map(|v| parse_comma_list(&v))
            .unwrap_or_default();

        let filter = NotificationFilter {
            instruction_types,
            config_types,
        };
        filter.validate()?;

        let display = DisplayConfig {
            show_permissions: reader
                .get_optional("SHOW_PERMISSIONS")
                .map(|v| parse_bool(&v))
                .unwrap_or(false),
            show_timestamps: reader
                .get_optional("SHOW_TIMESTAMPS")
                .map(|v| parse_bool(&v))
                .unwrap_or(false),
            truncate_addresses: reader
                .get_optional("TRUNCATE_ADDRESSES")
                .map(|v| parse_bool(&v))
                .unwrap_or(false),
        };

        let log_level = reader
            .get_optional("LOG_LEVEL")
            .map(|v| parse_log_level(&v))
            .unwrap_or(LogLevel::Error);

        let routes = reader
            .get_optional("MULTISIG_ROUTES")
            .map(|v| parse_routes(&v))
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

pub fn parse_routes(raw: &str) -> HashMap<String, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    struct MockEnv {
        vars: HashMap<String, String>,
    }

    impl MockEnv {
        fn new(vars: &[(&str, &str)]) -> Self {
            Self {
                vars: vars
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            }
        }
    }

    impl EnvReader for MockEnv {
        fn get_var(&self, name: &str) -> Result<String, MonitorError> {
            self.vars
                .get(name)
                .cloned()
                .ok_or_else(|| MonitorError::Config(format!("{name} not set")))
        }

        fn get_optional(&self, name: &str) -> Option<String> {
            self.vars.get(name).cloned()
        }
    }

    fn minimal_env() -> MockEnv {
        MockEnv::new(&[
            ("MULTISIG_ADDRESSES", "addr1,addr2"),
            ("HELIUS_AUTH_TOKEN", "token123"),
            ("SLACK_WEBHOOK_URL", "https://hooks.slack.com/test"),
        ])
    }

    #[test]
    fn from_reader_minimal_config() {
        let env = minimal_env();
        let config = Config::from_reader(&env).unwrap();
        assert_eq!(config.multisig_addresses, vec!["addr1", "addr2"]);
        assert_eq!(config.helius_auth_token, "token123");
        assert_eq!(config.log_level, LogLevel::Error);
        assert!(!config.display.show_permissions);
        assert!(config.routes.is_empty());
    }

    #[test]
    fn from_reader_with_optional_fields() {
        let env = MockEnv::new(&[
            ("MULTISIG_ADDRESSES", "addr1"),
            ("HELIUS_AUTH_TOKEN", "tok"),
            ("SLACK_WEBHOOK_URL", "https://hooks.slack.com/test"),
            ("SHOW_PERMISSIONS", "true"),
            ("LOG_LEVEL", "debug"),
            ("MULTISIG_ROUTES", "addr1:https://other.slack.com/hook"),
        ]);
        let config = Config::from_reader(&env).unwrap();
        assert!(config.display.show_permissions);
        assert_eq!(config.log_level, LogLevel::Debug);
        assert_eq!(
            config.resolve_webhook_url("addr1"),
            "https://other.slack.com/hook"
        );
        assert_eq!(
            config.resolve_webhook_url("unknown"),
            "https://hooks.slack.com/test"
        );
    }

    #[test]
    fn from_reader_missing_required_field() {
        let env = MockEnv::new(&[("MULTISIG_ADDRESSES", "addr1")]);
        let err = Config::from_reader(&env).unwrap_err();
        assert!(err.to_string().contains("HELIUS_AUTH_TOKEN"));
    }

    #[test]
    fn from_reader_empty_addresses() {
        let env = MockEnv::new(&[
            ("MULTISIG_ADDRESSES", ""),
            ("HELIUS_AUTH_TOKEN", "tok"),
            ("SLACK_WEBHOOK_URL", "url"),
        ]);
        let err = Config::from_reader(&env).unwrap_err();
        assert!(err.to_string().contains("MULTISIG_ADDRESSES is required"));
    }

    #[test]
    fn parse_routes_basic() {
        let routes = parse_routes("addr1:url1,addr2:url2");
        assert_eq!(routes.len(), 2);
        assert_eq!(routes["addr1"], "url1");
        assert_eq!(routes["addr2"], "url2");
    }

    #[test]
    fn parse_routes_empty() {
        assert!(parse_routes("").is_empty());
    }

    #[test]
    fn parse_routes_skips_malformed() {
        let routes = parse_routes("good:url,,bad,:also_bad,ok:fine");
        assert_eq!(routes.len(), 2);
        assert_eq!(routes["good"], "url");
        assert_eq!(routes["ok"], "fine");
    }

    #[test]
    fn resolve_webhook_url_falls_back_to_default() {
        let env = minimal_env();
        let config = Config::from_reader(&env).unwrap();
        assert_eq!(
            config.resolve_webhook_url("nonexistent"),
            "https://hooks.slack.com/test"
        );
    }
}

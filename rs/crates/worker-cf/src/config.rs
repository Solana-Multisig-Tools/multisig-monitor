use multisig_monitor_core::{
    config::{self, EnvReader},
    error::MonitorError,
};
use worker::Env;

pub type Config = config::Config;

const SECRETS: &[&str] = &["HELIUS_AUTH_TOKEN", "SLACK_WEBHOOK_URL"];

struct CfEnv<'a>(&'a Env);

impl EnvReader for CfEnv<'_> {
    fn get_var(&self, name: &str) -> Result<String, MonitorError> {
        let result = if SECRETS.contains(&name) {
            self.0.secret(name)
        } else {
            self.0.var(name)
        };
        result
            .map(|v| v.to_string())
            .map_err(|e| MonitorError::Config(format!("{name}: {e}")))
    }

    fn get_optional(&self, name: &str) -> Option<String> {
        self.0.var(name).ok().map(|v| v.to_string())
    }
}

pub fn from_env(env: &Env) -> Result<Config, MonitorError> {
    Config::from_reader(&CfEnv(env))
}

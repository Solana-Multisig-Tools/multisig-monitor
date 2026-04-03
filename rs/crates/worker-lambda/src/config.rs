use multisig_monitor_core::{
    config::{self, EnvReader},
    error::MonitorError,
};

pub type Config = config::Config;

struct StdEnv;

impl EnvReader for StdEnv {
    fn get_var(&self, name: &str) -> Result<String, MonitorError> {
        std::env::var(name).map_err(|_| MonitorError::Config(format!("{name} not set")))
    }

    fn get_optional(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}

pub fn from_env() -> Result<Config, MonitorError> {
    Config::from_reader(&StdEnv)
}

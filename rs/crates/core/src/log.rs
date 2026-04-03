use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Info,
    Debug,
}

impl LogLevel {
    fn ordinal(self) -> u8 {
        match self {
            Self::Error => 0,
            Self::Info => 1,
            Self::Debug => 2,
        }
    }
}

pub fn parse_log_level(value: &str) -> LogLevel {
    match value.to_lowercase().as_str() {
        "info" => LogLevel::Info,
        "debug" => LogLevel::Debug,
        _ => LogLevel::Error,
    }
}

#[derive(Serialize)]
struct LogEntry<'a> {
    level: &'a str,
    msg: &'a str,
    #[serde(flatten)]
    fields: Option<serde_json::Value>,
}

fn should_log(level: LogLevel, current: LogLevel) -> bool {
    level.ordinal() <= current.ordinal()
}

fn level_str(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Error => "error",
        LogLevel::Info => "info",
        LogLevel::Debug => "debug",
    }
}

pub fn log_json(
    level: LogLevel,
    current: LogLevel,
    msg: &str,
    fields: Option<serde_json::Value>,
) -> Option<String> {
    if !should_log(level, current) {
        return None;
    }
    let entry = LogEntry {
        level: level_str(level),
        msg,
        fields,
    };
    serde_json::to_string(&entry).ok()
}

#[derive(Debug, Clone, Default)]
pub struct DisplayConfig {
    pub show_permissions: bool,
    pub show_timestamps: bool,
    pub truncate_addresses: bool,
}

pub fn parse_bool(value: &str) -> bool {
    value.eq_ignore_ascii_case("true")
}

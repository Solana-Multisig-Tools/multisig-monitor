#[derive(Debug, thiserror::Error)]
pub enum MonitorError {
    #[error("unauthorized")]
    Unauthorized,

    #[error("payload parse failed: {0}")]
    PayloadParse(String),

    #[error("instruction data too short: {actual} < {expected}")]
    InstructionDataTooShort { expected: usize, actual: usize },

    #[error("config action decode failed: {0}")]
    ConfigActionDecode(String),

    #[error("slack send failed: status {status}")]
    SlackSend { status: u16 },

    #[error("serialization failed: {0}")]
    Serialization(String),

    #[error("platform error: {0}")]
    Platform(String),

    #[error("config error: {0}")]
    Config(String),
}

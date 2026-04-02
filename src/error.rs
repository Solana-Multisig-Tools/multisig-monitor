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

    #[error("worker: {0}")]
    Worker(String),
}

impl From<worker::Error> for MonitorError {
    fn from(err: worker::Error) -> Self {
        MonitorError::Worker(err.to_string())
    }
}

impl From<MonitorError> for worker::Error {
    fn from(err: MonitorError) -> Self {
        worker::Error::RustError(err.to_string())
    }
}

/// Truncates a base58 address to `first4...last4` for display.
pub fn truncate_address(addr: &str) -> String {
    if addr.len() <= 12 {
        return addr.to_string();
    }
    format!("{}...{}", &addr[..4], &addr[addr.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_address() {
        assert_eq!(truncate_address("7nYBkp3k"), "7nYBkp3k");
        assert_eq!(
            truncate_address("7nYB1234567890abcdef3kFd"),
            "7nYB...3kFd"
        );
    }
}

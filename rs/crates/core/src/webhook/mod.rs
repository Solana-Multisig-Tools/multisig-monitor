#[allow(dead_code)]
pub mod types;

pub use types::RawTransaction;

use crate::error::MonitorError;

/// Validates an authorization header value against the expected token.
/// Platform-specific code is responsible for extracting the header from the request.
pub fn validate_auth(auth_header: Option<&str>, expected_token: &str) -> Result<(), MonitorError> {
    let header = auth_header.ok_or(MonitorError::Unauthorized)?;
    if header != expected_token {
        return Err(MonitorError::Unauthorized);
    }
    Ok(())
}

/// Parses the raw webhook body into a vec of transactions.
/// Helius raw webhooks POST an array of transactions.
pub fn parse_payload(body: &str) -> Result<Vec<RawTransaction>, MonitorError> {
    serde_json::from_str(body).map_err(|e| MonitorError::PayloadParse(e.to_string()))
}

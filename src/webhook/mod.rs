#[allow(dead_code)]
pub mod types;

pub use types::RawTransaction;

use crate::error::MonitorError;
use worker::Request;

/// Validates the Authorization header against the expected token.
pub fn validate_auth(req: &Request, expected_token: &str) -> Result<(), MonitorError> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .ok()
        .flatten()
        .ok_or(MonitorError::Unauthorized)?;

    if auth_header != expected_token {
        return Err(MonitorError::Unauthorized);
    }

    Ok(())
}

/// Parses the raw webhook body into a vec of transactions.
/// Helius raw webhooks POST an array of transactions.
pub fn parse_payload(body: &str) -> Result<Vec<RawTransaction>, MonitorError> {
    serde_json::from_str(body).map_err(|e| MonitorError::PayloadParse(e.to_string()))
}

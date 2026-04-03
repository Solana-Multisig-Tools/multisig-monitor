#[allow(dead_code)]
pub mod types;

pub use types::RawTransaction;

use crate::error::MonitorError;
use subtle::ConstantTimeEq;

/// Validates an authorization header value against the expected token.
/// Uses constant-time comparison to prevent timing attacks.
pub fn validate_auth(auth_header: Option<&str>, expected_token: &str) -> Result<(), MonitorError> {
    let header = auth_header.ok_or(MonitorError::Unauthorized)?;
    if header.len() != expected_token.len()
        || header.as_bytes().ct_eq(expected_token.as_bytes()).unwrap_u8() != 1
    {
        return Err(MonitorError::Unauthorized);
    }
    Ok(())
}

/// Parses the raw webhook body into a vec of transactions.
/// Helius raw webhooks POST an array of transactions.
pub fn parse_payload(body: &str) -> Result<Vec<RawTransaction>, MonitorError> {
    serde_json::from_str(body).map_err(|e| MonitorError::PayloadParse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_auth_accepts_exact_match() {
        assert!(validate_auth(Some("secret-token"), "secret-token").is_ok());
    }

    #[test]
    fn validate_auth_rejects_missing_header() {
        assert!(matches!(
            validate_auth(None, "secret-token"),
            Err(MonitorError::Unauthorized)
        ));
    }

    #[test]
    fn validate_auth_rejects_length_mismatch() {
        assert!(matches!(
            validate_auth(Some("secret-token-extra"), "secret-token"),
            Err(MonitorError::Unauthorized)
        ));
    }

    #[test]
    fn validate_auth_rejects_same_length_wrong_value() {
        assert!(matches!(
            validate_auth(Some("secret-tokfn"), "secret-token"),
            Err(MonitorError::Unauthorized)
        ));
    }

    #[test]
    fn parse_payload_accepts_transaction_array() {
        let payload = r#"[{
            "signature": "sig-1",
            "transaction": {
                "message": {
                    "accountKeys": ["acct-1"],
                    "instructions": []
                },
                "signatures": ["sig-1"]
            },
            "meta": null
        }]"#;

        let parsed = parse_payload(payload).expect("payload should parse");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].signature(), Some("sig-1"));
    }

    #[test]
    fn parse_payload_rejects_non_array_json() {
        let payload = r#"{"signature":"sig-1"}"#;

        assert!(matches!(
            parse_payload(payload),
            Err(MonitorError::PayloadParse(_))
        ));
    }
}

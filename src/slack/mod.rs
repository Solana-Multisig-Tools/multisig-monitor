pub mod types;

use crate::error::{truncate_address, MonitorError};
use crate::squads::types::{DetectedOperation, SquadsOperation};
use types::{SlackBlock, SlackPayload, SlackText};
use worker::Fetch;

/// Builds a Slack Block Kit payload for a detected operation.
pub fn build_payload(op: &DetectedOperation) -> SlackPayload {
    let mut blocks = vec![
        SlackBlock::header(&format!(
            "{} {}",
            op.operation.emoji(),
            op.operation.display_name()
        )),
        SlackBlock::section_fields(vec![
            SlackText::mrkdwn(&format!(
                "*Multisig:*\n`{}`",
                truncate_address(&op.multisig)
            )),
            SlackText::mrkdwn(&format!(
                "*{}:*\n`{}`",
                op.operation.member_role(),
                truncate_address(&op.member)
            )),
        ]),
    ];

    // Add config action details if present
    if let SquadsOperation::ConfigTransactionCreate { actions } = &op.operation {
        if !actions.is_empty() {
            let action_lines: Vec<String> = actions
                .iter()
                .map(|a| format!("• {}", a.description()))
                .collect();

            blocks.push(SlackBlock::section_markdown(&format!(
                "*Config Actions:*\n{}",
                action_lines.join("\n")
            )));
        }
    }

    // Transaction signature with Solscan link
    blocks.push(SlackBlock::section_markdown(&format!(
        "*Tx:* <https://solscan.io/tx/{}|{}>",
        op.signature,
        truncate_address(&op.signature)
    )));

    blocks.push(SlackBlock::divider());

    SlackPayload { blocks }
}

/// Sends a Slack notification for a detected operation.
pub async fn send_notification(
    op: &DetectedOperation,
    webhook_url: &str,
) -> Result<(), MonitorError> {
    let payload = build_payload(op);
    let body =
        serde_json::to_string(&payload).map_err(|e| MonitorError::Worker(e.to_string()))?;

    let req = worker::Request::new_with_init(
        webhook_url,
        worker::RequestInit::new()
            .with_method(worker::Method::Post)
            .with_headers({
                let mut h = worker::Headers::new();
                h.set("Content-Type", "application/json")
                    .map_err(|e| MonitorError::Worker(e.to_string()))?;
                h
            })
            .with_body(Some(worker::wasm_bindgen::JsValue::from_str(&body))),
    )
    .map_err(|e| MonitorError::Worker(e.to_string()))?;

    let resp = Fetch::Request(req)
        .send()
        .await
        .map_err(|e| MonitorError::Worker(e.to_string()))?;

    match resp.status_code() {
        200 => Ok(()),
        status => Err(MonitorError::SlackSend { status }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::squads::types::{ConfigChange, Permissions};

    #[test]
    fn payload_for_proposal_approve() {
        let op = DetectedOperation {
            signature: "4fGhAbCdEfGhIjKlMnOpQrStUvWxYz1234567890abcd".to_string(),
            multisig: "7nYB1234567890abcdef3kFd".to_string(),
            member: "9xKz1234567890abcdefmR2p".to_string(),
            operation: SquadsOperation::ProposalApprove,
        };

        let payload = build_payload(&op);
        let json = serde_json::to_string_pretty(&payload).unwrap();
        assert!(json.contains("Proposal Approved"));
        assert!(json.contains("Approver"));
        assert!(json.contains("solscan.io"));
    }

    #[test]
    fn payload_for_config_transaction_with_actions() {
        let op = DetectedOperation {
            signature: "sigABC123".to_string(),
            multisig: "multisigXYZ".to_string(),
            member: "creatorABC".to_string(),
            operation: SquadsOperation::ConfigTransactionCreate {
                actions: vec![
                    ConfigChange::AddMember {
                        member: "newMember123".to_string(),
                        permissions: Permissions { mask: 0x06 },
                    },
                    ConfigChange::ChangeThreshold { new_threshold: 3 },
                ],
            },
        };

        let payload = build_payload(&op);
        let json = serde_json::to_string_pretty(&payload).unwrap();
        assert!(json.contains("Config Transaction Created"));
        assert!(json.contains("Config Actions"));
        assert!(json.contains("Add member"));
        assert!(json.contains("Vote, Execute"));
        assert!(json.contains("Change threshold to 3"));
    }
}

pub mod types;

use crate::error::{truncate_address, MonitorError};
use crate::squads::types::{DetectedOperation, SquadsOperation};
use types::{SlackBlock, SlackPayload, SlackText};

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

    // Transaction and proposal addresses for correlation
    if op.transaction_address.is_some() || op.proposal_address.is_some() {
        let mut fields = Vec::new();
        if let Some(ref tx_addr) = op.transaction_address {
            fields.push(SlackText::mrkdwn(&format!(
                "*Transaction:*\n`{}`",
                truncate_address(tx_addr)
            )));
        }
        if let Some(ref prop_addr) = op.proposal_address {
            fields.push(SlackText::mrkdwn(&format!(
                "*Proposal:*\n`{}`",
                truncate_address(prop_addr)
            )));
        }
        blocks.push(SlackBlock::section_fields(fields));
    }

    // Operation-specific detail sections
    append_operation_details(&mut blocks, &op.operation);

    // Transaction signature with Solscan link
    blocks.push(SlackBlock::section_markdown(&format!(
        "*Tx:* <https://solscan.io/tx/{}|{}>",
        op.signature,
        truncate_address(&op.signature)
    )));

    blocks.push(SlackBlock::divider());

    SlackPayload { blocks }
}

fn append_operation_details(blocks: &mut Vec<SlackBlock>, operation: &SquadsOperation) {
    match operation {
        SquadsOperation::ConfigTransactionCreate { actions } if !actions.is_empty() => {
            let lines: Vec<String> = actions
                .iter()
                .map(|a| format!("\u{2022} {}", a.description()))
                .collect();
            blocks.push(SlackBlock::section_markdown(&format!(
                "*Config Actions:*\n{}",
                lines.join("\n")
            )));
        }

        SquadsOperation::MultisigCreate {
            config_authority,
            threshold,
            members,
        } => {
            let member_lines: Vec<String> = members
                .iter()
                .map(|m| {
                    let perms = m.permissions.to_labels().join(", ");
                    format!("\u{2022} `{}` ({})", truncate_address(&m.key), perms)
                })
                .collect();
            let authority = config_authority
                .as_ref()
                .map(|a| truncate_address(a))
                .unwrap_or_else(|| "None (autonomous)".to_string());
            blocks.push(SlackBlock::section_markdown(&format!(
                "*Threshold:* {}\n*Config Authority:* {}\n*Members:*\n{}",
                threshold,
                authority,
                member_lines.join("\n")
            )));
        }

        SquadsOperation::DirectAddMember { new_member } => {
            let perms = new_member.permissions.to_labels().join(", ");
            blocks.push(SlackBlock::section_markdown(&format!(
                "*New Member:* `{}`\n*Permissions:* {}",
                truncate_address(&new_member.key),
                perms
            )));
        }
        SquadsOperation::DirectRemoveMember { old_member } => {
            blocks.push(SlackBlock::section_markdown(&format!(
                "*Removed Member:* `{}`",
                truncate_address(old_member)
            )));
        }
        SquadsOperation::DirectChangeThreshold { new_threshold } => {
            blocks.push(SlackBlock::section_markdown(&format!(
                "*New Threshold:* {}",
                new_threshold
            )));
        }
        SquadsOperation::DirectSetTimeLock { time_lock } => {
            blocks.push(SlackBlock::section_markdown(&format!(
                "*Time Lock:* {}s",
                time_lock
            )));
        }
        SquadsOperation::DirectSetConfigAuthority { config_authority } => {
            blocks.push(SlackBlock::section_markdown(&format!(
                "*New Config Authority:* `{}`",
                truncate_address(config_authority)
            )));
        }
        SquadsOperation::DirectSetRentCollector { rent_collector } => {
            let value = rent_collector
                .as_ref()
                .map(|a| format!("`{}`", truncate_address(a)))
                .unwrap_or_else(|| "None (removed)".to_string());
            blocks.push(SlackBlock::section_markdown(&format!(
                "*Rent Collector:* {}",
                value
            )));
        }
        SquadsOperation::DirectAddSpendingLimit {
            mint,
            amount,
            vault_index,
            ..
        } => {
            blocks.push(SlackBlock::section_markdown(&format!(
                "*Amount:* {}\n*Mint:* `{}`\n*Vault:* {}",
                amount,
                truncate_address(mint),
                vault_index
            )));
        }
        SquadsOperation::SpendingLimitUse { amount, decimals } => {
            blocks.push(SlackBlock::section_markdown(&format!(
                "*Amount:* {} (decimals: {})",
                amount, decimals
            )));
        }
        SquadsOperation::ProposalCreate { draft } if *draft => {
            blocks.push(SlackBlock::section_markdown("*Status:* Draft"));
        }
        _ => {}
    }
}

/// Serializes a Slack payload to JSON string, ready for HTTP POST.
pub fn serialize_payload(op: &DetectedOperation) -> Result<String, MonitorError> {
    let payload = build_payload(op);
    serde_json::to_string(&payload).map_err(|e| MonitorError::Serialization(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::squads::types::{ConfigChange, MemberEntry, Permissions};

    fn make_op(operation: SquadsOperation) -> DetectedOperation {
        DetectedOperation {
            signature: "4fGhAbCdEfGhIjKlMnOpQrStUvWxYz1234567890abcd".to_string(),
            multisig: "7nYB1234567890abcdef3kFd".to_string(),
            member: "9xKz1234567890abcdefmR2p".to_string(),
            transaction_address: None,
            proposal_address: None,
            operation,
        }
    }

    #[test]
    fn payload_for_proposal_approve() {
        let op = DetectedOperation {
            proposal_address: Some("prop1234567890abcdefABCD".to_string()),
            ..make_op(SquadsOperation::ProposalApprove)
        };
        let json = serde_json::to_string_pretty(&build_payload(&op)).unwrap();
        assert!(json.contains("Proposal Approved"));
        assert!(json.contains("Approver"));
        assert!(json.contains("Proposal"));
        assert!(json.contains("solscan.io"));
    }

    #[test]
    fn payload_for_config_transaction_with_actions() {
        let op = make_op(SquadsOperation::ConfigTransactionCreate {
            actions: vec![
                ConfigChange::AddMember {
                    member: "newMember123456789012".to_string(),
                    permissions: Permissions { mask: 0x06 },
                },
                ConfigChange::ChangeThreshold { new_threshold: 3 },
            ],
        });
        let json = serde_json::to_string_pretty(&build_payload(&op)).unwrap();
        assert!(json.contains("Config Transaction Created"));
        assert!(json.contains("Add member"));
        assert!(json.contains("Change threshold to 3"));
    }

    #[test]
    fn payload_for_direct_add_member() {
        let op = make_op(SquadsOperation::DirectAddMember {
            new_member: MemberEntry {
                key: "newKey1234567890123456".to_string(),
                permissions: Permissions { mask: 0x07 },
            },
        });
        let json = serde_json::to_string_pretty(&build_payload(&op)).unwrap();
        assert!(json.contains("Direct: Member Added"));
        assert!(json.contains("Config Authority"));
        assert!(json.contains("Initiate, Vote, Execute"));
    }

    #[test]
    fn payload_includes_transaction_and_proposal_addresses() {
        let op = DetectedOperation {
            transaction_address: Some("txAddr1234567890123456".to_string()),
            proposal_address: Some("propAddr1234567890abcde".to_string()),
            ..make_op(SquadsOperation::VaultTransactionExecute)
        };
        let json = serde_json::to_string_pretty(&build_payload(&op)).unwrap();
        assert!(json.contains("Transaction"));
        assert!(json.contains("Proposal"));
        assert!(json.contains("txAd...3456"));
        assert!(json.contains("prop...bcde"));
    }
}

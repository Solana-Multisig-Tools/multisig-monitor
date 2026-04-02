pub mod types;

use crate::error::MonitorError;
use crate::squads::types::{DetectedOperation, SquadsOperation};
use types::{SlackBlock, SlackPayload};

/// Builds a Slack Block Kit payload for a detected operation.
pub fn build_payload(op: &DetectedOperation) -> SlackPayload {
    let mut blocks = vec![
        SlackBlock::header(&format!(
            "{} {}",
            op.operation.emoji(),
            op.operation.display_name()
        )),
        SlackBlock::section_markdown(&format!(
            "*Multisig:* `{}`\n*{}:* `{}`",
            op.multisig,
            op.operation.member_role(),
            op.member
        )),
    ];

    // Transaction and proposal addresses for correlation
    let mut ref_lines = Vec::new();
    if let Some(ref tx_addr) = op.transaction_address {
        ref_lines.push(format!("*Transaction:* `{tx_addr}`"));
    }
    if let Some(ref prop_addr) = op.proposal_address {
        ref_lines.push(format!("*Proposal:* `{prop_addr}`"));
    }
    if !ref_lines.is_empty() {
        blocks.push(SlackBlock::section_markdown(&ref_lines.join("\n")));
    }

    // Operation-specific detail sections
    append_operation_details(&mut blocks, &op.operation);

    // Transaction signature with Solscan link
    blocks.push(SlackBlock::section_markdown(&format!(
        "*Tx:* <https://solscan.io/tx/{sig}|{sig}>",
        sig = op.signature
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
                    format!("\u{2022} `{}` ({})", m.key, perms)
                })
                .collect();
            let authority = config_authority
                .as_deref()
                .unwrap_or("None (autonomous)");
            blocks.push(SlackBlock::section_markdown(&format!(
                "*Threshold:* {threshold}\n*Config Authority:* {authority}\n*Members:*\n{}",
                member_lines.join("\n")
            )));
        }

        SquadsOperation::DirectAddMember { new_member } => {
            let perms = new_member.permissions.to_labels().join(", ");
            blocks.push(SlackBlock::section_markdown(&format!(
                "*New Member:* `{}`\n*Permissions:* {}",
                new_member.key, perms
            )));
        }
        SquadsOperation::DirectRemoveMember { old_member } => {
            blocks.push(SlackBlock::section_markdown(&format!(
                "*Removed Member:* `{old_member}`"
            )));
        }
        SquadsOperation::DirectChangeThreshold { new_threshold } => {
            blocks.push(SlackBlock::section_markdown(&format!(
                "*New Threshold:* {new_threshold}"
            )));
        }
        SquadsOperation::DirectSetTimeLock { time_lock } => {
            blocks.push(SlackBlock::section_markdown(&format!(
                "*Time Lock:* {time_lock}s"
            )));
        }
        SquadsOperation::DirectSetConfigAuthority { config_authority } => {
            blocks.push(SlackBlock::section_markdown(&format!(
                "*New Config Authority:* `{config_authority}`"
            )));
        }
        SquadsOperation::DirectSetRentCollector { rent_collector } => {
            let value = rent_collector
                .as_ref()
                .map(|a| format!("`{a}`"))
                .unwrap_or_else(|| "None (removed)".to_string());
            blocks.push(SlackBlock::section_markdown(&format!(
                "*Rent Collector:* {value}"
            )));
        }
        SquadsOperation::DirectAddSpendingLimit {
            mint,
            amount,
            vault_index,
            ..
        } => {
            blocks.push(SlackBlock::section_markdown(&format!(
                "*Amount:* {amount}\n*Mint:* `{mint}`\n*Vault:* {vault_index}"
            )));
        }
        SquadsOperation::SpendingLimitUse { amount, decimals } => {
            blocks.push(SlackBlock::section_markdown(&format!(
                "*Amount:* {amount} (decimals: {decimals})"
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
            signature: "5nNtjezQMYBHvgSQmoRmJPiXGsPAWmJPoGSa64xanqrauogiVzFyGQhKeFataHGXq51jR2hjbzNTkPUpP787HAmL".to_string(),
            multisig: "SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc".to_string(),
            member: "9xKzmR2pLn4sHj7wBcDfAe8qYt6vXkZ3nPo1uWr5mQjS".to_string(),
            transaction_address: None,
            proposal_address: None,
            operation,
        }
    }

    #[test]
    fn payload_shows_full_addresses() {
        let op = make_op(SquadsOperation::ProposalApprove);
        let json = serde_json::to_string(&build_payload(&op)).unwrap();
        // Full addresses, not truncated
        assert!(json.contains("SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc"));
        assert!(json.contains("9xKzmR2pLn4sHj7wBcDfAe8qYt6vXkZ3nPo1uWr5mQjS"));
        // Key and value on same line (no newline between label and value)
        assert!(json.contains("*Multisig:* `SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc`"));
        assert!(json.contains("*Approver:* `9xKzmR2pLn4sHj7wBcDfAe8qYt6vXkZ3nPo1uWr5mQjS`"));
    }

    #[test]
    fn payload_for_proposal_approve() {
        let op = DetectedOperation {
            proposal_address: Some("Prop1ABCDEFGHijk2345678901234567890abcdefgh".to_string()),
            ..make_op(SquadsOperation::ProposalApprove)
        };
        let json = serde_json::to_string_pretty(&build_payload(&op)).unwrap();
        assert!(json.contains("Proposal Approved"));
        assert!(json.contains("Approver"));
        assert!(json.contains("Prop1ABCDEFGHijk2345678901234567890abcdefgh"));
        assert!(json.contains("solscan.io"));
    }

    #[test]
    fn payload_for_config_transaction_with_actions() {
        let op = make_op(SquadsOperation::ConfigTransactionCreate {
            actions: vec![
                ConfigChange::AddMember {
                    member: "HjK4NewMember567890abcdefghijklmnopqrstuv12".to_string(),
                    permissions: Permissions { mask: 0x06 },
                },
                ConfigChange::ChangeThreshold { new_threshold: 3 },
            ],
        });
        let json = serde_json::to_string_pretty(&build_payload(&op)).unwrap();
        assert!(json.contains("Config Transaction Created"));
        assert!(json.contains("HjK4NewMember567890abcdefghijklmnopqrstuv12"));
        assert!(json.contains("Change threshold to 3"));
    }

    #[test]
    fn payload_for_direct_add_member() {
        let op = make_op(SquadsOperation::DirectAddMember {
            new_member: MemberEntry {
                key: "NewKeyABCDEF1234567890abcdefghijklmnopqrstuv".to_string(),
                permissions: Permissions { mask: 0x07 },
            },
        });
        let json = serde_json::to_string_pretty(&build_payload(&op)).unwrap();
        assert!(json.contains("Direct: Member Added"));
        assert!(json.contains("Config Authority"));
        assert!(json.contains("NewKeyABCDEF1234567890abcdefghijklmnopqrstuv"));
        assert!(json.contains("Initiate, Vote, Execute"));
    }

    #[test]
    fn payload_includes_full_transaction_and_proposal_addresses() {
        let op = DetectedOperation {
            transaction_address: Some("TxAddr1234567890abcdefghijklmnopqrstuvwxyz12".to_string()),
            proposal_address: Some("PropAddr567890abcdefghijklmnopqrstuvwxyz1234".to_string()),
            ..make_op(SquadsOperation::VaultTransactionExecute)
        };
        let json = serde_json::to_string_pretty(&build_payload(&op)).unwrap();
        assert!(json.contains("TxAddr1234567890abcdefghijklmnopqrstuvwxyz12"));
        assert!(json.contains("PropAddr567890abcdefghijklmnopqrstuvwxyz1234"));
    }

    #[test]
    fn signature_link_shows_full_signature() {
        let op = make_op(SquadsOperation::ProposalApprove);
        let json = serde_json::to_string(&build_payload(&op)).unwrap();
        let sig = "5nNtjezQMYBHvgSQmoRmJPiXGsPAWmJPoGSa64xanqrauogiVzFyGQhKeFataHGXq51jR2hjbzNTkPUpP787HAmL";
        assert!(json.contains(&format!("solscan.io/tx/{sig}|{sig}")));
    }
}

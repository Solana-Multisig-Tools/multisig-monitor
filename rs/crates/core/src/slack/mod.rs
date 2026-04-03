pub mod types;

use crate::error::MonitorError;
use crate::log::DisplayConfig;
use crate::squads::types::{DetectedOperation, SquadsOperation, truncate_address};
use types::{SlackBlock, SlackPayload};

fn format_address(address: &str, display: &DisplayConfig) -> String {
    if display.truncate_addresses {
        format!("{} `{}`", truncate_address(address), address)
    } else {
        format!("`{}`", address)
    }
}

fn format_timestamp(block_time: i64) -> String {
    // Convert Unix timestamp to UTC datetime string
    let total_secs = block_time;
    let secs_of_day = ((total_secs % 86400) + 86400) % 86400;
    let hh = secs_of_day / 3600;
    let mm = (secs_of_day % 3600) / 60;

    // Civil date from Unix days using the algorithm from Howard Hinnant
    let days = total_secs.div_euclid(86400) + 719468;
    let era = days.div_euclid(146097);
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02} UTC")
}

/// Builds a Slack Block Kit payload for a detected operation.
pub fn build_payload(op: &DetectedOperation, display: &DisplayConfig) -> SlackPayload {
    let mut blocks = vec![
        SlackBlock::header(&format!(
            "{} {}",
            op.operation.emoji(),
            op.operation.display_name()
        )),
        SlackBlock::section_markdown(&format!(
            "*Multisig:* {}\n*{}:* {}",
            format_address(&op.multisig, display),
            op.operation.member_role(),
            format_address(&op.member, display)
        )),
    ];

    // Transaction and proposal addresses for correlation
    let mut ref_lines = Vec::new();
    if let Some(ref tx_addr) = op.transaction_address {
        ref_lines.push(format!(
            "*Transaction:* {}",
            format_address(tx_addr, display)
        ));
    }
    if let Some(ref prop_addr) = op.proposal_address {
        ref_lines.push(format!(
            "*Proposal:* {}",
            format_address(prop_addr, display)
        ));
    }
    if !ref_lines.is_empty() {
        blocks.push(SlackBlock::section_markdown(&ref_lines.join("\n")));
    }

    // Operation-specific detail sections
    append_operation_details(&mut blocks, &op.operation, display);

    // Transaction signature with Solscan link
    blocks.push(SlackBlock::section_markdown(&format!(
        "*Tx:* <https://solscan.io/tx/{sig}|{sig}>",
        sig = op.signature
    )));

    if display.show_timestamps
        && let Some(bt) = op.block_time
    {
        blocks.push(SlackBlock::section_markdown(&format!(
            "*Time:* {}",
            format_timestamp(bt)
        )));
    }

    blocks.push(SlackBlock::divider());

    SlackPayload { blocks }
}

fn append_operation_details(
    blocks: &mut Vec<SlackBlock>,
    operation: &SquadsOperation,
    display: &DisplayConfig,
) {
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
                    format!("\u{2022} {} ({})", format_address(&m.key, display), perms)
                })
                .collect();
            let authority = match config_authority {
                Some(a) => format_address(a, display),
                None => "None (autonomous)".to_string(),
            };
            blocks.push(SlackBlock::section_markdown(&format!(
                "*Threshold:* {threshold}\n*Config Authority:* {}\n*Members:*\n{}",
                authority,
                member_lines.join("\n")
            )));
        }

        SquadsOperation::DirectAddMember { new_member } => {
            let perms = new_member.permissions.to_labels().join(", ");
            blocks.push(SlackBlock::section_markdown(&format!(
                "*New Member:* {}\n*Permissions:* {}",
                format_address(&new_member.key, display),
                perms
            )));
        }
        SquadsOperation::DirectRemoveMember { old_member } => {
            blocks.push(SlackBlock::section_markdown(&format!(
                "*Removed Member:* {}",
                format_address(old_member, display)
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
                "*New Config Authority:* {}",
                format_address(config_authority, display)
            )));
        }
        SquadsOperation::DirectSetRentCollector { rent_collector } => {
            let value = rent_collector
                .as_ref()
                .map(|a| format_address(a, display))
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
                "*Amount:* {amount}\n*Mint:* {}\n*Vault:* {vault_index}",
                format_address(mint, display)
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
pub fn serialize_payload(
    op: &DetectedOperation,
    display: &DisplayConfig,
) -> Result<String, MonitorError> {
    let payload = build_payload(op, display);
    serde_json::to_string(&payload).map_err(|e| MonitorError::Serialization(e.to_string()))
}

const MAX_BLOCKS_PER_MESSAGE: usize = 50;

/// Builds one or more Slack payloads from a batch of operations.
/// Chunks into multiple messages if blocks exceed Slack's 50-block limit.
pub fn build_batch_payload(
    ops: &[DetectedOperation],
    display: &DisplayConfig,
) -> Vec<SlackPayload> {
    if ops.is_empty() {
        return vec![];
    }

    let all_blocks: Vec<SlackBlock> = ops
        .iter()
        .flat_map(|op| build_payload(op, display).blocks)
        .collect();

    all_blocks
        .chunks(MAX_BLOCKS_PER_MESSAGE)
        .map(|chunk| SlackPayload {
            blocks: chunk.to_vec(),
        })
        .collect()
}

/// Serializes a batch of operations into JSON payloads ready for HTTP POST.
pub fn serialize_batch_payloads(
    ops: &[DetectedOperation],
    display: &DisplayConfig,
) -> Result<Vec<String>, MonitorError> {
    build_batch_payload(ops, display)
        .iter()
        .map(|p| serde_json::to_string(p).map_err(|e| MonitorError::Serialization(e.to_string())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::DisplayConfig;
    use crate::squads::types::{ConfigChange, MemberEntry, Permissions};

    fn make_op(operation: SquadsOperation) -> DetectedOperation {
        DetectedOperation {
            signature: "5nNtjezQMYBHvgSQmoRmJPiXGsPAWmJPoGSa64xanqrauogiVzFyGQhKeFataHGXq51jR2hjbzNTkPUpP787HAmL".to_string(),
            multisig: "SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc".to_string(),
            member: "9xKzmR2pLn4sHj7wBcDfAe8qYt6vXkZ3nPo1uWr5mQjS".to_string(),
            transaction_address: None,
            proposal_address: None,
            operation,
            block_time: None,
        }
    }

    #[test]
    fn payload_shows_full_addresses() {
        let op = make_op(SquadsOperation::ProposalApprove);
        let json = serde_json::to_string(&build_payload(&op, &DisplayConfig::default())).unwrap();
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
        let json =
            serde_json::to_string_pretty(&build_payload(&op, &DisplayConfig::default())).unwrap();
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
        let json =
            serde_json::to_string_pretty(&build_payload(&op, &DisplayConfig::default())).unwrap();
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
        let json =
            serde_json::to_string_pretty(&build_payload(&op, &DisplayConfig::default())).unwrap();
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
        let json =
            serde_json::to_string_pretty(&build_payload(&op, &DisplayConfig::default())).unwrap();
        assert!(json.contains("TxAddr1234567890abcdefghijklmnopqrstuvwxyz12"));
        assert!(json.contains("PropAddr567890abcdefghijklmnopqrstuvwxyz1234"));
    }

    #[test]
    fn signature_link_shows_full_signature() {
        let op = make_op(SquadsOperation::ProposalApprove);
        let json = serde_json::to_string(&build_payload(&op, &DisplayConfig::default())).unwrap();
        let sig = "5nNtjezQMYBHvgSQmoRmJPiXGsPAWmJPoGSa64xanqrauogiVzFyGQhKeFataHGXq51jR2hjbzNTkPUpP787HAmL";
        assert!(json.contains(&format!("solscan.io/tx/{sig}|{sig}")));
    }

    #[test]
    fn payload_truncates_addresses_when_configured() {
        let display = DisplayConfig {
            truncate_addresses: true,
            ..Default::default()
        };
        let op = make_op(SquadsOperation::ProposalApprove);
        let json = serde_json::to_string(&build_payload(&op, &display)).unwrap();
        // Truncated form present
        assert!(json.contains("SMPL...EjXc"));
        // Full address still in code block
        assert!(json.contains("SMPLDaGKqbPfi8NhZMNGH2fRYU3WbNRZVj3xnTjEjXc"));
    }

    #[test]
    fn payload_shows_timestamp_when_configured() {
        let display = DisplayConfig {
            show_timestamps: true,
            ..Default::default()
        };
        let op = DetectedOperation {
            block_time: Some(1712150400),
            ..make_op(SquadsOperation::ProposalApprove)
        };
        let json = serde_json::to_string(&build_payload(&op, &display)).unwrap();
        assert!(json.contains("*Time:*"));
        assert!(json.contains("UTC"));
    }

    #[test]
    fn batch_payload_combines_operations() {
        let op1 = make_op(SquadsOperation::ProposalApprove);
        let op2 = make_op(SquadsOperation::ProposalReject);
        let payloads = build_batch_payload(&[op1, op2], &DisplayConfig::default());
        assert_eq!(payloads.len(), 1);
        let json = serde_json::to_string(&payloads[0]).unwrap();
        assert!(json.contains("Proposal Approved"));
        assert!(json.contains("Proposal Rejected"));
    }

    #[test]
    fn batch_payload_empty_for_no_ops() {
        let payloads = build_batch_payload(&[], &DisplayConfig::default());
        assert!(payloads.is_empty());
    }
}

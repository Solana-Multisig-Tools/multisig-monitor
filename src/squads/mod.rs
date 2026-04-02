pub mod accounts;
pub mod config_action;
pub mod discriminator;
pub mod types;

use std::collections::HashMap;

use crate::webhook::types::{CompiledInstruction, RawTransaction};
use accounts::{layout_for, resolve_pubkey};
use config_action::decode_config_actions;
use discriminator::{build_discriminator_map, match_discriminator, InstructionKind};
use types::{DetectedOperation, SquadsOperation, SQUADS_PROGRAM_ID};

/// Extracts all Squads v4 operations from a batch of raw transactions,
/// filtered to only the configured multisig addresses.
pub fn extract_operations(
    transactions: &[RawTransaction],
    monitored_multisigs: &[String],
) -> Vec<DetectedOperation> {
    let disc_map = build_discriminator_map();

    transactions
        .iter()
        .filter(|tx| tx.meta.as_ref().is_none_or(|m| m.err.is_none()))
        .flat_map(|tx| extract_from_transaction(tx, monitored_multisigs, &disc_map))
        .collect()
}

fn extract_from_transaction(
    tx: &RawTransaction,
    monitored_multisigs: &[String],
    disc_map: &HashMap<[u8; 8], InstructionKind>,
) -> Vec<DetectedOperation> {
    let account_keys = &tx.transaction.message.account_keys;
    let signature = &tx.signature;

    let top_level = tx
        .transaction
        .message
        .instructions
        .iter()
        .filter_map(|instr| {
            process_instruction(instr, account_keys, signature, monitored_multisigs, disc_map)
        });

    let inner = tx
        .meta
        .as_ref()
        .and_then(|m| m.inner_instructions.as_ref())
        .into_iter()
        .flatten()
        .flat_map(|set| &set.instructions)
        .filter_map(|instr| {
            process_instruction(instr, account_keys, signature, monitored_multisigs, disc_map)
        });

    top_level.chain(inner).collect()
}

fn process_instruction(
    instr: &CompiledInstruction,
    account_keys: &[String],
    signature: &str,
    monitored_multisigs: &[String],
    disc_map: &HashMap<[u8; 8], InstructionKind>,
) -> Option<DetectedOperation> {
    // Check if this instruction targets the Squads v4 program
    let program_id = account_keys.get(instr.program_id_index as usize)?;
    if program_id != SQUADS_PROGRAM_ID {
        return None;
    }

    // Decode instruction data from base58
    let data = bs58::decode(&instr.data).into_vec().ok()?;

    // Match the discriminator
    let kind = match_discriminator(&data, disc_map)?;

    // Resolve the multisig and member accounts
    let layout = layout_for(kind);
    let multisig = resolve_pubkey(&instr.accounts, account_keys, layout.multisig_index)?;

    // Only process monitored multisigs
    if !monitored_multisigs.contains(&multisig) {
        return None;
    }

    let member = resolve_pubkey(&instr.accounts, account_keys, layout.member_index)?;

    // Build the operation, decoding config actions if applicable
    let operation = match kind {
        InstructionKind::ConfigTransactionCreate => {
            let actions = decode_config_actions(&data).unwrap_or_default();
            SquadsOperation::ConfigTransactionCreate { actions }
        }
        InstructionKind::ProposalCreate => SquadsOperation::ProposalCreate,
        InstructionKind::ProposalActivate => SquadsOperation::ProposalActivate,
        InstructionKind::ProposalApprove => SquadsOperation::ProposalApprove,
        InstructionKind::ProposalReject => SquadsOperation::ProposalReject,
        InstructionKind::ProposalCancel => SquadsOperation::ProposalCancel,
        InstructionKind::VaultTransactionCreate => SquadsOperation::VaultTransactionCreate,
        InstructionKind::VaultTransactionExecute => SquadsOperation::VaultTransactionExecute,
        InstructionKind::ConfigTransactionExecute => SquadsOperation::ConfigTransactionExecute,
    };

    Some(DetectedOperation {
        signature: signature.to_string(),
        multisig,
        member,
        operation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webhook::types::*;

    fn make_squads_instruction(
        kind: InstructionKind,
        account_keys: &[&str],
    ) -> (CompiledInstruction, Vec<String>) {
        // Build account_keys vec with program ID included
        let mut keys: Vec<String> = account_keys.iter().map(|s| s.to_string()).collect();
        let program_idx = keys.len() as u8;
        keys.push(SQUADS_PROGRAM_ID.to_string());

        // Build discriminator data
        let disc_map = build_discriminator_map();
        let disc = disc_map
            .iter()
            .find(|(_, &v)| v == kind)
            .map(|(k, _)| *k)
            .unwrap();

        let mut data = disc.to_vec();
        // For config_transaction_create, append borsh-encoded empty Vec<ConfigAction> + None memo
        if kind == InstructionKind::ConfigTransactionCreate {
            data.extend_from_slice(&[0, 0, 0, 0]); // empty vec length
            data.push(0); // None memo
        }

        let accounts: Vec<u8> = (0..account_keys.len() as u8).collect();

        let instr = CompiledInstruction {
            program_id_index: program_idx,
            accounts,
            data: bs58::encode(&data).into_string(),
        };

        (instr, keys)
    }

    #[test]
    fn extract_proposal_approve() {
        let (instr, keys) =
            make_squads_instruction(InstructionKind::ProposalApprove, &["multisig1", "member1", "proposal1"]);

        let tx = RawTransaction {
            signature: "sig123".to_string(),
            slot: 100,
            block_time: None,
            transaction: TransactionEnvelope {
                message: TransactionMessage {
                    account_keys: keys,
                    instructions: vec![instr],
                },
                signatures: vec!["sig123".to_string()],
            },
            meta: Some(TransactionMeta {
                err: None,
                inner_instructions: None,
                log_messages: None,
            }),
        };

        let ops = extract_operations(&[tx], &["multisig1".to_string()]);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].multisig, "multisig1");
        assert_eq!(ops[0].member, "member1");
        assert_eq!(ops[0].operation, SquadsOperation::ProposalApprove);
    }

    #[test]
    fn skips_failed_transactions() {
        let (instr, keys) =
            make_squads_instruction(InstructionKind::ProposalApprove, &["multisig1", "member1", "proposal1"]);

        let tx = RawTransaction {
            signature: "sig_fail".to_string(),
            slot: 100,
            block_time: None,
            transaction: TransactionEnvelope {
                message: TransactionMessage {
                    account_keys: keys,
                    instructions: vec![instr],
                },
                signatures: vec!["sig_fail".to_string()],
            },
            meta: Some(TransactionMeta {
                err: Some(serde_json::json!({"InstructionError": [0, "Custom"]})),
                inner_instructions: None,
                log_messages: None,
            }),
        };

        let ops = extract_operations(&[tx], &["multisig1".to_string()]);
        assert!(ops.is_empty(), "failed transactions should be skipped");
    }

    #[test]
    fn skips_unmonitored_multisig() {
        let (instr, keys) =
            make_squads_instruction(InstructionKind::ProposalApprove, &["other_multisig", "member1", "proposal1"]);

        let tx = RawTransaction {
            signature: "sig456".to_string(),
            slot: 100,
            block_time: None,
            transaction: TransactionEnvelope {
                message: TransactionMessage {
                    account_keys: keys,
                    instructions: vec![instr],
                },
                signatures: vec!["sig456".to_string()],
            },
            meta: Some(TransactionMeta {
                err: None,
                inner_instructions: None,
                log_messages: None,
            }),
        };

        let ops = extract_operations(&[tx], &["multisig1".to_string()]);
        assert!(ops.is_empty(), "unmonitored multisig should be skipped");
    }
}

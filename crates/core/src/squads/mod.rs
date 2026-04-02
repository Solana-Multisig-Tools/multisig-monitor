pub mod accounts;
pub mod config_action;
pub mod discriminator;
pub mod types;

use std::collections::HashMap;

use crate::filter::NotificationFilter;
use crate::webhook::types::{CompiledInstruction, RawTransaction};
use accounts::{layout_for, resolve_pubkey};
use config_action::*;
use discriminator::{build_discriminator_map, match_discriminator, InstructionKind};
use types::{DetectedOperation, MemberEntry, Permissions, SquadsOperation, SQUADS_PROGRAM_ID};

/// Extracts all Squads v4 operations from a batch of raw transactions,
/// filtered to only the configured multisig addresses and matching the notification filter.
pub fn extract_operations(
    transactions: &[RawTransaction],
    monitored_multisigs: &[String],
    filter: &NotificationFilter,
) -> Vec<DetectedOperation> {
    let disc_map = build_discriminator_map();

    transactions
        .iter()
        .filter(|tx| tx.meta.as_ref().is_none_or(|m| m.err.is_none()))
        .flat_map(|tx| extract_from_transaction(tx, monitored_multisigs, &disc_map, filter))
        .collect()
}

fn extract_from_transaction(
    tx: &RawTransaction,
    monitored_multisigs: &[String],
    disc_map: &HashMap<[u8; 8], InstructionKind>,
    filter: &NotificationFilter,
) -> Vec<DetectedOperation> {
    let signature = match tx.signature() {
        Some(s) => s.to_string(),
        None => return vec![],
    };

    // Resolve AccountKeyEntry (plain string or structured object) to plain pubkey strings
    let account_keys: Vec<String> = tx
        .transaction
        .message
        .account_keys
        .iter()
        .map(|k| k.pubkey().to_string())
        .collect();

    let top_level = tx
        .transaction
        .message
        .instructions
        .iter()
        .filter_map(|instr| {
            process_instruction(
                instr,
                &account_keys,
                &signature,
                monitored_multisigs,
                disc_map,
                filter,
            )
        });

    let inner = tx
        .meta
        .as_ref()
        .and_then(|m| m.inner_instructions.as_ref())
        .into_iter()
        .flatten()
        .flat_map(|set| &set.instructions)
        .filter_map(|instr| {
            process_instruction(
                instr,
                &account_keys,
                &signature,
                monitored_multisigs,
                disc_map,
                filter,
            )
        });

    top_level.chain(inner).collect()
}

fn process_instruction(
    instr: &CompiledInstruction,
    account_keys: &[String],
    signature: &str,
    monitored_multisigs: &[String],
    disc_map: &HashMap<[u8; 8], InstructionKind>,
    filter: &NotificationFilter,
) -> Option<DetectedOperation> {
    let program_id = account_keys.get(instr.program_id_index as usize)?;
    if program_id != SQUADS_PROGRAM_ID {
        return None;
    }

    let data = bs58::decode(&instr.data).into_vec().ok()?;
    let kind = match_discriminator(&data, disc_map)?;

    if !filter.allows_instruction(kind) {
        return None;
    }

    let layout = layout_for(kind);
    let multisig = resolve_pubkey(&instr.accounts, account_keys, layout.multisig_index)?;

    if !monitored_multisigs.contains(&multisig) {
        return None;
    }

    let member = resolve_pubkey(&instr.accounts, account_keys, layout.member_index)?;

    let transaction_address = layout
        .transaction_index
        .and_then(|idx| resolve_pubkey(&instr.accounts, account_keys, idx));

    let proposal_address = layout
        .proposal_index
        .and_then(|idx| resolve_pubkey(&instr.accounts, account_keys, idx));

    let operation = build_operation(kind, &data)?;
    let operation = filter.apply(operation)?;

    Some(DetectedOperation {
        signature: signature.to_string(),
        multisig,
        member,
        transaction_address,
        proposal_address,
        operation,
    })
}

/// Builds a SquadsOperation from the instruction kind and raw data.
/// Decodes args where relevant; falls back gracefully on decode failures.
fn build_operation(kind: InstructionKind, data: &[u8]) -> Option<SquadsOperation> {
    let op = match kind {
        // -- Multisig creation --
        InstructionKind::MultisigCreate => SquadsOperation::MultisigCreate {
            config_authority: None,
            threshold: 0,
            members: vec![],
        },
        InstructionKind::MultisigCreateV2 => {
            let args = decode_multisig_create_v2(data).ok()?;
            SquadsOperation::MultisigCreate {
                config_authority: args.config_authority.map(|p| p.to_base58()),
                threshold: args.threshold,
                members: args.members.iter().map(|m| m.to_member_entry()).collect(),
            }
        }

        // -- Proposal lifecycle --
        InstructionKind::ProposalCreate => {
            let draft = decode_proposal_create(data)
                .map(|a| a.draft)
                .unwrap_or(false);
            SquadsOperation::ProposalCreate { draft }
        }
        InstructionKind::ProposalActivate => SquadsOperation::ProposalActivate,
        InstructionKind::ProposalApprove => SquadsOperation::ProposalApprove,
        InstructionKind::ProposalReject => SquadsOperation::ProposalReject,
        InstructionKind::ProposalCancel => SquadsOperation::ProposalCancel,
        InstructionKind::ProposalCancelV2 => SquadsOperation::ProposalCancelV2,

        // -- Transaction creation --
        InstructionKind::VaultTransactionCreate => SquadsOperation::VaultTransactionCreate,
        InstructionKind::ConfigTransactionCreate => {
            let actions = decode_config_actions(data).unwrap_or_default();
            SquadsOperation::ConfigTransactionCreate { actions }
        }

        // -- Transaction execution --
        InstructionKind::VaultTransactionExecute => SquadsOperation::VaultTransactionExecute,
        InstructionKind::ConfigTransactionExecute => SquadsOperation::ConfigTransactionExecute,

        // -- Controlled multisig direct config --
        InstructionKind::MultisigAddMember => {
            let entry = decode_multisig_add_member(data).unwrap_or(MemberEntry {
                key: "unknown".to_string(),
                permissions: Permissions { mask: 0 },
            });
            SquadsOperation::DirectAddMember { new_member: entry }
        }
        InstructionKind::MultisigRemoveMember => {
            let old = decode_multisig_remove_member(data).unwrap_or_else(|_| "unknown".to_string());
            SquadsOperation::DirectRemoveMember { old_member: old }
        }
        InstructionKind::MultisigChangeThreshold => {
            let t = decode_multisig_change_threshold(data).unwrap_or(0);
            SquadsOperation::DirectChangeThreshold { new_threshold: t }
        }
        InstructionKind::MultisigSetTimeLock => {
            let t = decode_multisig_set_time_lock(data).unwrap_or(0);
            SquadsOperation::DirectSetTimeLock { time_lock: t }
        }
        InstructionKind::MultisigSetConfigAuthority => {
            let a = decode_multisig_set_config_authority(data)
                .unwrap_or_else(|_| "unknown".to_string());
            SquadsOperation::DirectSetConfigAuthority {
                config_authority: a,
            }
        }
        InstructionKind::MultisigSetRentCollector => {
            let c = decode_multisig_set_rent_collector(data).unwrap_or(None);
            SquadsOperation::DirectSetRentCollector { rent_collector: c }
        }
        InstructionKind::MultisigAddSpendingLimit => {
            match decode_multisig_add_spending_limit(data) {
                Ok(args) => SquadsOperation::DirectAddSpendingLimit {
                    create_key: args.create_key.to_base58(),
                    vault_index: args.vault_index,
                    mint: args.mint.to_base58(),
                    amount: args.amount,
                    members: args.members.iter().map(|p| p.to_base58()).collect(),
                    destinations: args.destinations.iter().map(|p| p.to_base58()).collect(),
                },
                Err(_) => SquadsOperation::DirectAddSpendingLimit {
                    create_key: "unknown".to_string(),
                    vault_index: 0,
                    mint: "unknown".to_string(),
                    amount: 0,
                    members: vec![],
                    destinations: vec![],
                },
            }
        }
        InstructionKind::MultisigRemoveSpendingLimit => SquadsOperation::DirectRemoveSpendingLimit,

        // -- Spending limit usage --
        InstructionKind::SpendingLimitUse => {
            let (amount, decimals) = decode_spending_limit_use(data)
                .map(|a| (a.amount, a.decimals))
                .unwrap_or((0, 0));
            SquadsOperation::SpendingLimitUse { amount, decimals }
        }

        // -- Buffer operations --
        InstructionKind::TransactionBufferCreate => SquadsOperation::TransactionBufferCreate,
        InstructionKind::TransactionBufferExtend => SquadsOperation::TransactionBufferExtend,
        InstructionKind::TransactionBufferClose => SquadsOperation::TransactionBufferClose,
        InstructionKind::VaultTransactionCreateFromBuffer => {
            SquadsOperation::VaultTransactionCreateFromBuffer
        }

        // -- Batch operations --
        InstructionKind::BatchCreate => SquadsOperation::BatchCreate,
        InstructionKind::BatchAddTransaction => SquadsOperation::BatchAddTransaction,
        InstructionKind::BatchExecuteTransaction => SquadsOperation::BatchExecuteTransaction,

        // -- Account cleanup --
        InstructionKind::ConfigTransactionAccountsClose => {
            SquadsOperation::ConfigTransactionAccountsClose
        }
        InstructionKind::VaultTransactionAccountsClose => {
            SquadsOperation::VaultTransactionAccountsClose
        }
        InstructionKind::VaultBatchTransactionAccountClose => {
            SquadsOperation::VaultBatchTransactionAccountClose
        }
        InstructionKind::BatchAccountsClose => SquadsOperation::BatchAccountsClose,
    };

    Some(op)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webhook::types::*;

    fn make_squads_instruction(
        kind: InstructionKind,
        account_keys: &[&str],
    ) -> (CompiledInstruction, Vec<AccountKeyEntry>) {
        let mut keys: Vec<AccountKeyEntry> = account_keys
            .iter()
            .map(|s| AccountKeyEntry::Plain(s.to_string()))
            .collect();
        let program_idx = keys.len() as u8;
        keys.push(AccountKeyEntry::Plain(SQUADS_PROGRAM_ID.to_string()));

        let disc_map = build_discriminator_map();
        let disc = disc_map
            .iter()
            .find(|(_, &v)| v == kind)
            .map(|(k, _)| *k)
            .unwrap();

        let mut data = disc.to_vec();
        match kind {
            InstructionKind::ConfigTransactionCreate => {
                data.extend_from_slice(&[0, 0, 0, 0]); // empty vec
                data.push(0); // None memo
            }
            InstructionKind::ProposalCreate => {
                data.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0]); // transaction_index = 1
                data.push(0); // draft = false
            }
            _ => {}
        }

        let accounts: Vec<u8> = (0..account_keys.len() as u8).collect();

        let instr = CompiledInstruction {
            program_id_index: program_idx,
            accounts,
            data: bs58::encode(&data).into_string(),
        };

        (instr, keys)
    }

    fn make_tx(kind: InstructionKind, accounts: &[&str], sig: &str) -> RawTransaction {
        let (instr, keys) = make_squads_instruction(kind, accounts);
        RawTransaction {
            signature: None, // mimic real Helius raw payloads (no top-level signature)
            slot: 100,
            block_time: None,
            index_within_block: None,
            transaction: TransactionEnvelope {
                message: TransactionMessage {
                    account_keys: keys,
                    instructions: vec![instr],
                },
                signatures: vec![sig.to_string()], // signature lives here
            },
            meta: Some(TransactionMeta {
                err: None,
                inner_instructions: None,
                log_messages: None,
            }),
        }
    }

    #[test]
    fn extract_proposal_approve_with_addresses() {
        // proposal_approve: [multisig, member✍, proposal]
        let tx = make_tx(
            InstructionKind::ProposalApprove,
            &["multisig1", "member1", "proposal1"],
            "sig123",
        );

        let ops = extract_operations(
            &[tx],
            &["multisig1".to_string()],
            &NotificationFilter::default(),
        );
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].multisig, "multisig1");
        assert_eq!(ops[0].member, "member1");
        assert_eq!(ops[0].proposal_address, Some("proposal1".to_string()));
        assert_eq!(ops[0].transaction_address, None);
        assert_eq!(ops[0].operation, SquadsOperation::ProposalApprove);
    }

    #[test]
    fn extract_vault_tx_execute_with_addresses() {
        // vault_transaction_execute: [multisig, proposal, transaction, member✍]
        let tx = make_tx(
            InstructionKind::VaultTransactionExecute,
            &["multisig1", "proposal1", "tx_account1", "executor1"],
            "sig_exec",
        );

        let ops = extract_operations(
            &[tx],
            &["multisig1".to_string()],
            &NotificationFilter::default(),
        );
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].member, "executor1");
        assert_eq!(ops[0].proposal_address, Some("proposal1".to_string()));
        assert_eq!(ops[0].transaction_address, Some("tx_account1".to_string()));
    }

    #[test]
    fn extract_direct_config_change() {
        // multisig_add_member: [multisig, configAuthority✍, ...]
        let tx = make_tx(
            InstructionKind::MultisigAddMember,
            &["multisig1", "authority1", "payer1", "system1"],
            "sig_direct",
        );

        let ops = extract_operations(
            &[tx],
            &["multisig1".to_string()],
            &NotificationFilter::default(),
        );
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].member, "authority1");
        assert!(matches!(
            ops[0].operation,
            SquadsOperation::DirectAddMember { .. }
        ));
    }

    #[test]
    fn skips_failed_transactions() {
        let (instr, keys) = make_squads_instruction(
            InstructionKind::ProposalApprove,
            &["multisig1", "member1", "proposal1"],
        );
        let tx = RawTransaction {
            signature: None,
            slot: 100,
            block_time: None,
            index_within_block: None,
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

        let ops = extract_operations(
            &[tx],
            &["multisig1".to_string()],
            &NotificationFilter::default(),
        );
        assert!(ops.is_empty());
    }

    #[test]
    fn instruction_type_filter_excludes_unmatched() {
        let tx = make_tx(
            InstructionKind::ProposalApprove,
            &["multisig1", "member1", "proposal1"],
            "sig789",
        );

        let filter = NotificationFilter {
            instruction_types: vec!["proposal_reject".to_string()],
            ..Default::default()
        };

        let ops = extract_operations(&[tx], &["multisig1".to_string()], &filter);
        assert!(ops.is_empty());
    }
}

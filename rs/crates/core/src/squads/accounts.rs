use super::discriminator::InstructionKind;

/// Defines the account index positions for each Squads v4 instruction.
/// All indices reference positions in the instruction's `accounts` array
/// (which itself contains indices into the transaction's top-level `account_keys`).
pub struct AccountLayout {
    pub multisig_index: usize,
    pub member_index: usize,
    /// Transaction account, if present. Used for correlating creates with executions.
    pub transaction_index: Option<usize>,
    /// Proposal account, if present.
    pub proposal_index: Option<usize>,
}

/// Returns the account layout for a given instruction kind.
/// Derived from the Squads v4 Anchor IDL account ordering.
pub fn layout_for(kind: InstructionKind) -> AccountLayout {
    match kind {
        // -- Multisig creation --
        // multisig_create: deprecated, minimal accounts. [multisig, createKey✍, ...]
        InstructionKind::MultisigCreate => AccountLayout {
            multisig_index: 0,
            member_index: 1,
            transaction_index: None,
            proposal_index: None,
        },
        // multisig_create_v2: [programConfig, treasury, multisig, createKey✍, creator✍, systemProgram]
        InstructionKind::MultisigCreateV2 => AccountLayout {
            multisig_index: 2,
            member_index: 4,
            transaction_index: None,
            proposal_index: None,
        },

        // -- Proposal lifecycle --
        // proposal_create: [multisig, proposal, creator✍, rentPayer, systemProgram]
        InstructionKind::ProposalCreate => AccountLayout {
            multisig_index: 0,
            member_index: 2,
            transaction_index: None,
            proposal_index: Some(1),
        },
        // proposal_activate/approve/reject/cancel: [multisig, member✍, proposal]
        InstructionKind::ProposalActivate
        | InstructionKind::ProposalApprove
        | InstructionKind::ProposalReject
        | InstructionKind::ProposalCancel => AccountLayout {
            multisig_index: 0,
            member_index: 1,
            transaction_index: None,
            proposal_index: Some(2),
        },
        // proposal_cancel_v2: nested [multisig, member✍, proposal], systemProgram
        InstructionKind::ProposalCancelV2 => AccountLayout {
            multisig_index: 0,
            member_index: 1,
            transaction_index: None,
            proposal_index: Some(2),
        },

        // -- Transaction creation --
        // vault_transaction_create: [multisig, transaction, creator✍, rentPayer, systemProgram]
        // config_transaction_create: [multisig, transaction, creator✍, rentPayer, systemProgram]
        InstructionKind::VaultTransactionCreate | InstructionKind::ConfigTransactionCreate => {
            AccountLayout {
                multisig_index: 0,
                member_index: 2,
                transaction_index: Some(1),
                proposal_index: None,
            }
        }

        // -- Transaction execution --
        // vault_transaction_execute: [multisig, proposal, transaction, member✍, ...]
        InstructionKind::VaultTransactionExecute => AccountLayout {
            multisig_index: 0,
            member_index: 3,
            transaction_index: Some(2),
            proposal_index: Some(1),
        },
        // config_transaction_execute: [multisig, member✍, proposal, transaction, ...]
        InstructionKind::ConfigTransactionExecute => AccountLayout {
            multisig_index: 0,
            member_index: 1,
            transaction_index: Some(3),
            proposal_index: Some(2),
        },

        // -- Controlled multisig direct config --
        // All follow: [multisig, configAuthority✍, ...]
        InstructionKind::MultisigAddMember
        | InstructionKind::MultisigRemoveMember
        | InstructionKind::MultisigChangeThreshold
        | InstructionKind::MultisigSetTimeLock
        | InstructionKind::MultisigSetConfigAuthority
        | InstructionKind::MultisigSetRentCollector
        | InstructionKind::MultisigAddSpendingLimit
        | InstructionKind::MultisigRemoveSpendingLimit => AccountLayout {
            multisig_index: 0,
            member_index: 1,
            transaction_index: None,
            proposal_index: None,
        },

        // -- Spending limit usage --
        // spending_limit_use: [multisig, member✍, spendingLimit, vault, destination, ...]
        InstructionKind::SpendingLimitUse => AccountLayout {
            multisig_index: 0,
            member_index: 1,
            transaction_index: None,
            proposal_index: None,
        },

        // -- Buffer-based transaction creation --
        // transaction_buffer_create: [multisig, transactionBuffer, creator✍, rentPayer, systemProgram]
        // transaction_buffer_extend: [multisig, transactionBuffer, creator✍]
        // transaction_buffer_close: [multisig, transactionBuffer, creator✍]
        InstructionKind::TransactionBufferCreate
        | InstructionKind::TransactionBufferExtend
        | InstructionKind::TransactionBufferClose => AccountLayout {
            multisig_index: 0,
            member_index: 2,
            transaction_index: None,
            proposal_index: None,
        },
        // vault_transaction_create_from_buffer: nested [multisig, transaction, creator✍, rentPayer, systemProgram], transactionBuffer, creator✍
        InstructionKind::VaultTransactionCreateFromBuffer => AccountLayout {
            multisig_index: 0,
            member_index: 2,
            transaction_index: Some(1),
            proposal_index: None,
        },

        // -- Batch operations --
        // batch_create: [multisig, batch, creator✍, rentPayer, systemProgram]
        InstructionKind::BatchCreate => AccountLayout {
            multisig_index: 0,
            member_index: 2,
            transaction_index: None,
            proposal_index: None,
        },
        // batch_add_transaction: [multisig, proposal, batch, transaction, member✍, rentPayer, systemProgram]
        InstructionKind::BatchAddTransaction => AccountLayout {
            multisig_index: 0,
            member_index: 4,
            transaction_index: Some(3),
            proposal_index: Some(1),
        },
        // batch_execute_transaction: [multisig, member✍, proposal, batch, transaction, ...]
        InstructionKind::BatchExecuteTransaction => AccountLayout {
            multisig_index: 0,
            member_index: 1,
            transaction_index: Some(4),
            proposal_index: Some(2),
        },

        // -- Account cleanup (no signer member — use rentCollector as actor) --
        // config_transaction_accounts_close: [multisig, proposal, transaction, rentCollector, systemProgram]
        // vault_transaction_accounts_close: [multisig, proposal, transaction, rentCollector, systemProgram]
        InstructionKind::ConfigTransactionAccountsClose
        | InstructionKind::VaultTransactionAccountsClose => AccountLayout {
            multisig_index: 0,
            member_index: 3,
            transaction_index: Some(2),
            proposal_index: Some(1),
        },
        // vault_batch_transaction_account_close: [multisig, proposal, batch, transaction, rentCollector, systemProgram]
        InstructionKind::VaultBatchTransactionAccountClose => AccountLayout {
            multisig_index: 0,
            member_index: 4,
            transaction_index: Some(3),
            proposal_index: Some(1),
        },
        // batch_accounts_close: [multisig, proposal, batch, rentCollector, systemProgram]
        InstructionKind::BatchAccountsClose => AccountLayout {
            multisig_index: 0,
            member_index: 3,
            transaction_index: None,
            proposal_index: Some(1),
        },
    }
}

/// Resolves a pubkey from instruction account indices and the top-level account_keys.
pub fn resolve_pubkey(
    instruction_accounts: &[u8],
    account_keys: &[String],
    position: usize,
) -> Option<String> {
    instruction_accounts
        .get(position)
        .and_then(|&idx| account_keys.get(idx as usize))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_pubkey_works() {
        let account_keys = vec![
            "multisig111".to_string(),
            "member222".to_string(),
            "proposal333".to_string(),
        ];
        let instr_accounts: Vec<u8> = vec![0, 1, 2];

        assert_eq!(
            resolve_pubkey(&instr_accounts, &account_keys, 0),
            Some("multisig111".to_string())
        );
        assert_eq!(
            resolve_pubkey(&instr_accounts, &account_keys, 1),
            Some("member222".to_string())
        );
    }

    #[test]
    fn resolve_pubkey_out_of_bounds() {
        let account_keys = vec!["only_one".to_string()];
        let instr_accounts: Vec<u8> = vec![0, 5];
        assert_eq!(resolve_pubkey(&instr_accounts, &account_keys, 1), None);
    }
}

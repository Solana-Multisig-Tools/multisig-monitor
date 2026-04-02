use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Represents every Squads v4 instruction we monitor.
/// Excludes the 4 program-wide programConfig* instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstructionKind {
    // Multisig creation
    MultisigCreate,
    MultisigCreateV2,

    // Proposal lifecycle
    ProposalCreate,
    ProposalActivate,
    ProposalApprove,
    ProposalReject,
    ProposalCancel,
    ProposalCancelV2,

    // Transaction creation
    VaultTransactionCreate,
    ConfigTransactionCreate,

    // Transaction execution
    VaultTransactionExecute,
    ConfigTransactionExecute,

    // Controlled multisig direct config
    MultisigAddMember,
    MultisigRemoveMember,
    MultisigChangeThreshold,
    MultisigSetTimeLock,
    MultisigSetConfigAuthority,
    MultisigSetRentCollector,
    MultisigAddSpendingLimit,
    MultisigRemoveSpendingLimit,

    // Spending limit usage
    SpendingLimitUse,

    // Buffer-based transaction creation
    TransactionBufferCreate,
    TransactionBufferExtend,
    TransactionBufferClose,
    VaultTransactionCreateFromBuffer,

    // Batch operations
    BatchCreate,
    BatchAddTransaction,
    BatchExecuteTransaction,

    // Account cleanup
    ConfigTransactionAccountsClose,
    VaultTransactionAccountsClose,
    VaultBatchTransactionAccountClose,
    BatchAccountsClose,
}

impl InstructionKind {
    /// The Anchor instruction name used for discriminator computation.
    pub fn anchor_name(self) -> &'static str {
        match self {
            Self::MultisigCreate => "multisig_create",
            Self::MultisigCreateV2 => "multisig_create_v2",
            Self::ProposalCreate => "proposal_create",
            Self::ProposalActivate => "proposal_activate",
            Self::ProposalApprove => "proposal_approve",
            Self::ProposalReject => "proposal_reject",
            Self::ProposalCancel => "proposal_cancel",
            Self::ProposalCancelV2 => "proposal_cancel_v2",
            Self::VaultTransactionCreate => "vault_transaction_create",
            Self::ConfigTransactionCreate => "config_transaction_create",
            Self::VaultTransactionExecute => "vault_transaction_execute",
            Self::ConfigTransactionExecute => "config_transaction_execute",
            Self::MultisigAddMember => "multisig_add_member",
            Self::MultisigRemoveMember => "multisig_remove_member",
            Self::MultisigChangeThreshold => "multisig_change_threshold",
            Self::MultisigSetTimeLock => "multisig_set_time_lock",
            Self::MultisigSetConfigAuthority => "multisig_set_config_authority",
            Self::MultisigSetRentCollector => "multisig_set_rent_collector",
            Self::MultisigAddSpendingLimit => "multisig_add_spending_limit",
            Self::MultisigRemoveSpendingLimit => "multisig_remove_spending_limit",
            Self::SpendingLimitUse => "spending_limit_use",
            Self::TransactionBufferCreate => "transaction_buffer_create",
            Self::TransactionBufferExtend => "transaction_buffer_extend",
            Self::TransactionBufferClose => "transaction_buffer_close",
            Self::VaultTransactionCreateFromBuffer => "vault_transaction_create_from_buffer",
            Self::BatchCreate => "batch_create",
            Self::BatchAddTransaction => "batch_add_transaction",
            Self::BatchExecuteTransaction => "batch_execute_transaction",
            Self::ConfigTransactionAccountsClose => "config_transaction_accounts_close",
            Self::VaultTransactionAccountsClose => "vault_transaction_accounts_close",
            Self::VaultBatchTransactionAccountClose => "vault_batch_transaction_account_close",
            Self::BatchAccountsClose => "batch_accounts_close",
        }
    }

    /// All instruction kinds we monitor.
    pub fn all() -> &'static [InstructionKind] {
        &[
            Self::MultisigCreate,
            Self::MultisigCreateV2,
            Self::ProposalCreate,
            Self::ProposalActivate,
            Self::ProposalApprove,
            Self::ProposalReject,
            Self::ProposalCancel,
            Self::ProposalCancelV2,
            Self::VaultTransactionCreate,
            Self::ConfigTransactionCreate,
            Self::VaultTransactionExecute,
            Self::ConfigTransactionExecute,
            Self::MultisigAddMember,
            Self::MultisigRemoveMember,
            Self::MultisigChangeThreshold,
            Self::MultisigSetTimeLock,
            Self::MultisigSetConfigAuthority,
            Self::MultisigSetRentCollector,
            Self::MultisigAddSpendingLimit,
            Self::MultisigRemoveSpendingLimit,
            Self::SpendingLimitUse,
            Self::TransactionBufferCreate,
            Self::TransactionBufferExtend,
            Self::TransactionBufferClose,
            Self::VaultTransactionCreateFromBuffer,
            Self::BatchCreate,
            Self::BatchAddTransaction,
            Self::BatchExecuteTransaction,
            Self::ConfigTransactionAccountsClose,
            Self::VaultTransactionAccountsClose,
            Self::VaultBatchTransactionAccountClose,
            Self::BatchAccountsClose,
        ]
    }
}

/// Computes the 8-byte Anchor discriminator: `sha256("global:<name>")[..8]`.
fn compute_discriminator(instruction_name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{instruction_name}").as_bytes());
    let hash = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

/// Builds a lookup table from 8-byte discriminator to InstructionKind.
pub fn build_discriminator_map() -> HashMap<[u8; 8], InstructionKind> {
    InstructionKind::all()
        .iter()
        .map(|&kind| (compute_discriminator(kind.anchor_name()), kind))
        .collect()
}

/// Looks up an instruction kind from the first 8 bytes of decoded instruction data.
pub fn match_discriminator(
    data: &[u8],
    map: &HashMap<[u8; 8], InstructionKind>,
) -> Option<InstructionKind> {
    if data.len() < 8 {
        return None;
    }
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&data[..8]);
    map.get(&disc).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discriminators_are_unique() {
        let map = build_discriminator_map();
        assert_eq!(
            map.len(),
            InstructionKind::all().len(),
            "all instructions should have unique discriminators"
        );
    }

    #[test]
    fn discriminator_is_deterministic() {
        let d1 = compute_discriminator("proposal_approve");
        let d2 = compute_discriminator("proposal_approve");
        assert_eq!(d1, d2);
    }

    #[test]
    fn match_returns_none_for_short_data() {
        let map = build_discriminator_map();
        assert_eq!(match_discriminator(&[0u8; 4], &map), None);
    }

    #[test]
    fn match_returns_none_for_unknown_discriminator() {
        let map = build_discriminator_map();
        assert_eq!(match_discriminator(&[0u8; 8], &map), None);
    }

    #[test]
    fn match_returns_correct_kind() {
        let map = build_discriminator_map();
        let disc = compute_discriminator("proposal_approve");
        let mut data = disc.to_vec();
        data.extend_from_slice(&[0u8; 16]);
        assert_eq!(
            match_discriminator(&data, &map),
            Some(InstructionKind::ProposalApprove)
        );
    }
}

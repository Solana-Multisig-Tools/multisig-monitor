use super::discriminator::InstructionKind;

/// Defines the account index positions for each Squads v4 instruction.
/// These are derived from the Anchor IDL account ordering.
pub struct AccountLayout {
    /// Index into the instruction's accounts array for the multisig PDA (always 0).
    pub multisig_index: usize,
    /// Index into the instruction's accounts array for the acting member/creator.
    pub member_index: usize,
}

/// Returns the account layout for a given instruction kind.
///
/// Account layouts from the Squads v4 Anchor IDL:
/// - proposal_create:             [multisig, proposal, creator✍, rent_payer, system_program]
/// - proposal_activate:           [multisig, member✍, proposal]
/// - proposal_approve:            [multisig, member✍, proposal]
/// - proposal_reject:             [multisig, member✍, proposal]
/// - proposal_cancel:             [multisig, member✍, proposal]
/// - vault_transaction_create:    [multisig, transaction, creator✍, rent_payer, system_program]
/// - config_transaction_create:   [multisig, transaction, creator✍, rent_payer, system_program]
/// - vault_transaction_execute:   [multisig, proposal, transaction, member✍, ...]
/// - config_transaction_execute:  [multisig, member✍, proposal, transaction, ...]
pub fn layout_for(kind: InstructionKind) -> AccountLayout {
    match kind {
        // member at index 2 (creator position)
        InstructionKind::ProposalCreate
        | InstructionKind::VaultTransactionCreate
        | InstructionKind::ConfigTransactionCreate => AccountLayout {
            multisig_index: 0,
            member_index: 2,
        },

        // member at index 1
        InstructionKind::ProposalActivate
        | InstructionKind::ProposalApprove
        | InstructionKind::ProposalReject
        | InstructionKind::ProposalCancel
        | InstructionKind::ConfigTransactionExecute => AccountLayout {
            multisig_index: 0,
            member_index: 1,
        },

        // member at index 3
        InstructionKind::VaultTransactionExecute => AccountLayout {
            multisig_index: 0,
            member_index: 3,
        },
    }
}

/// Resolves a pubkey from instruction account indices and the top-level account_keys.
/// Returns None if any index is out of bounds.
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
        // instruction accounts reference indices into account_keys
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
        let instr_accounts: Vec<u8> = vec![0, 5]; // index 5 is out of bounds
        assert_eq!(resolve_pubkey(&instr_accounts, &account_keys, 1), None);
    }
}

use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Represents the set of Squads v4 instructions we monitor.
/// Each variant maps to an Anchor instruction name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstructionKind {
    ProposalCreate,
    ProposalActivate,
    ProposalApprove,
    ProposalReject,
    ProposalCancel,
    VaultTransactionCreate,
    ConfigTransactionCreate,
    VaultTransactionExecute,
    ConfigTransactionExecute,
}

impl InstructionKind {
    /// The Anchor instruction name used for discriminator computation.
    fn anchor_name(self) -> &'static str {
        match self {
            Self::ProposalCreate => "proposal_create",
            Self::ProposalActivate => "proposal_activate",
            Self::ProposalApprove => "proposal_approve",
            Self::ProposalReject => "proposal_reject",
            Self::ProposalCancel => "proposal_cancel",
            Self::VaultTransactionCreate => "vault_transaction_create",
            Self::ConfigTransactionCreate => "config_transaction_create",
            Self::VaultTransactionExecute => "vault_transaction_execute",
            Self::ConfigTransactionExecute => "config_transaction_execute",
        }
    }

    /// All instruction kinds we monitor.
    fn all() -> &'static [InstructionKind] {
        &[
            Self::ProposalCreate,
            Self::ProposalActivate,
            Self::ProposalApprove,
            Self::ProposalReject,
            Self::ProposalCancel,
            Self::VaultTransactionCreate,
            Self::ConfigTransactionCreate,
            Self::VaultTransactionExecute,
            Self::ConfigTransactionExecute,
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
        assert_eq!(map.len(), 9, "all 9 instructions should have unique discriminators");
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
        data.extend_from_slice(&[0u8; 16]); // extra bytes (args)
        assert_eq!(
            match_discriminator(&data, &map),
            Some(InstructionKind::ProposalApprove)
        );
    }
}

use borsh::BorshDeserialize;
#[cfg(test)]
use borsh::BorshSerialize;

use crate::error::MonitorError;
use crate::squads::types::{ConfigChange, MemberEntry, Permissions};

// ---------------------------------------------------------------------------
// Raw on-chain types (Borsh layout)
// ---------------------------------------------------------------------------

/// Raw 32-byte public key as stored on-chain.
#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
pub struct RawPubkey(pub [u8; 32]);

impl RawPubkey {
    pub fn to_base58(&self) -> String {
        bs58::encode(&self.0).into_string()
    }
}

/// On-chain Member struct: pubkey + permissions bitmask.
#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
pub struct RawMember {
    pub key: RawPubkey,
    pub permissions: u8,
}

impl RawMember {
    pub fn to_member_entry(&self) -> MemberEntry {
        MemberEntry {
            key: self.key.to_base58(),
            permissions: Permissions {
                mask: self.permissions,
            },
        }
    }
}

/// On-chain Period enum for spending limits.
#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
pub enum RawPeriod {
    OneTime,
    Day,
    Week,
    Month,
}

/// On-chain ConfigAction enum, Borsh-serialized in config_transaction_create args.
#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
#[allow(dead_code)]
pub enum RawConfigAction {
    AddMember {
        new_member: RawMember,
    },
    RemoveMember {
        old_member: RawPubkey,
    },
    ChangeThreshold {
        new_threshold: u16,
    },
    SetTimeLock {
        new_time_lock: u32,
    },
    AddSpendingLimit {
        create_key: RawPubkey,
        vault_index: u8,
        mint: RawPubkey,
        amount: u64,
        period: RawPeriod,
        members: Vec<RawPubkey>,
        destinations: Vec<RawPubkey>,
    },
    RemoveSpendingLimit {
        spending_limit: RawPubkey,
    },
    SetRentCollector {
        new_rent_collector: Option<RawPubkey>,
    },
}

// ---------------------------------------------------------------------------
// Arg structs for each decoded instruction (after 8-byte discriminator)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
#[allow(dead_code)]
pub struct ConfigTransactionCreateArgs {
    pub actions: Vec<RawConfigAction>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
#[allow(dead_code)]
pub struct MultisigAddMemberArgs {
    pub new_member: RawMember,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
#[allow(dead_code)]
pub struct MultisigRemoveMemberArgs {
    pub old_member: RawPubkey,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
#[allow(dead_code)]
pub struct MultisigChangeThresholdArgs {
    pub new_threshold: u16,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
#[allow(dead_code)]
pub struct MultisigSetTimeLockArgs {
    pub time_lock: u32,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
#[allow(dead_code)]
pub struct MultisigSetConfigAuthorityArgs {
    pub config_authority: RawPubkey,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
#[allow(dead_code)]
pub struct MultisigSetRentCollectorArgs {
    pub rent_collector: Option<RawPubkey>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
#[allow(dead_code)]
pub struct MultisigAddSpendingLimitArgs {
    pub create_key: RawPubkey,
    pub vault_index: u8,
    pub mint: RawPubkey,
    pub amount: u64,
    pub period: RawPeriod,
    pub members: Vec<RawPubkey>,
    pub destinations: Vec<RawPubkey>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
#[allow(dead_code)]
pub struct MultisigCreateArgsV2 {
    pub config_authority: Option<RawPubkey>,
    pub threshold: u16,
    pub members: Vec<RawMember>,
    pub time_lock: u32,
    pub rent_collector: Option<RawPubkey>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
#[allow(dead_code)]
pub struct SpendingLimitUseArgs {
    pub amount: u64,
    pub decimals: u8,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
#[allow(dead_code)]
pub struct ProposalCreateArgs {
    pub transaction_index: u64,
    pub draft: bool,
}

// ---------------------------------------------------------------------------
// Decoders — each skips the 8-byte discriminator, then Borsh-deserializes
// ---------------------------------------------------------------------------

fn skip_discriminator(data: &[u8]) -> Result<&[u8], MonitorError> {
    if data.len() < 8 {
        return Err(MonitorError::InstructionDataTooShort {
            expected: 8,
            actual: data.len(),
        });
    }
    Ok(&data[8..])
}

fn decode_args<T: BorshDeserialize>(data: &[u8]) -> Result<T, MonitorError> {
    let args_data = skip_discriminator(data)?;
    T::try_from_slice(args_data).map_err(|e| MonitorError::ConfigActionDecode(e.to_string()))
}

pub fn decode_config_actions(data: &[u8]) -> Result<Vec<ConfigChange>, MonitorError> {
    let args: ConfigTransactionCreateArgs = decode_args(data)?;
    Ok(args.actions.into_iter().map(raw_to_config_change).collect())
}

pub fn decode_multisig_add_member(data: &[u8]) -> Result<MemberEntry, MonitorError> {
    let args: MultisigAddMemberArgs = decode_args(data)?;
    Ok(args.new_member.to_member_entry())
}

pub fn decode_multisig_remove_member(data: &[u8]) -> Result<String, MonitorError> {
    let args: MultisigRemoveMemberArgs = decode_args(data)?;
    Ok(args.old_member.to_base58())
}

pub fn decode_multisig_change_threshold(data: &[u8]) -> Result<u16, MonitorError> {
    let args: MultisigChangeThresholdArgs = decode_args(data)?;
    Ok(args.new_threshold)
}

pub fn decode_multisig_set_time_lock(data: &[u8]) -> Result<u32, MonitorError> {
    let args: MultisigSetTimeLockArgs = decode_args(data)?;
    Ok(args.time_lock)
}

pub fn decode_multisig_set_config_authority(data: &[u8]) -> Result<String, MonitorError> {
    let args: MultisigSetConfigAuthorityArgs = decode_args(data)?;
    Ok(args.config_authority.to_base58())
}

pub fn decode_multisig_set_rent_collector(data: &[u8]) -> Result<Option<String>, MonitorError> {
    let args: MultisigSetRentCollectorArgs = decode_args(data)?;
    Ok(args.rent_collector.map(|p| p.to_base58()))
}

pub fn decode_multisig_add_spending_limit(
    data: &[u8],
) -> Result<MultisigAddSpendingLimitArgs, MonitorError> {
    decode_args(data)
}

pub fn decode_multisig_create_v2(data: &[u8]) -> Result<MultisigCreateArgsV2, MonitorError> {
    decode_args(data)
}

pub fn decode_spending_limit_use(data: &[u8]) -> Result<SpendingLimitUseArgs, MonitorError> {
    decode_args(data)
}

pub fn decode_proposal_create(data: &[u8]) -> Result<ProposalCreateArgs, MonitorError> {
    decode_args(data)
}

fn raw_to_config_change(raw: RawConfigAction) -> ConfigChange {
    match raw {
        RawConfigAction::AddMember { new_member } => ConfigChange::AddMember {
            member: new_member.key.to_base58(),
            permissions: Permissions {
                mask: new_member.permissions,
            },
        },
        RawConfigAction::RemoveMember { old_member } => ConfigChange::RemoveMember {
            member: old_member.to_base58(),
        },
        RawConfigAction::ChangeThreshold { new_threshold } => {
            ConfigChange::ChangeThreshold { new_threshold }
        }
        RawConfigAction::SetTimeLock { new_time_lock } => ConfigChange::SetTimeLock {
            seconds: new_time_lock,
        },
        RawConfigAction::AddSpendingLimit {
            create_key,
            vault_index,
            mint,
            amount,
            members,
            destinations,
            ..
        } => ConfigChange::AddSpendingLimit {
            create_key: create_key.to_base58(),
            vault_index,
            mint: mint.to_base58(),
            amount,
            members: members.iter().map(|p| p.to_base58()).collect(),
            destinations: destinations.iter().map(|p| p.to_base58()).collect(),
        },
        RawConfigAction::RemoveSpendingLimit { spending_limit } => {
            ConfigChange::RemoveSpendingLimit {
                spending_limit: spending_limit.to_base58(),
            }
        }
        RawConfigAction::SetRentCollector { new_rent_collector } => {
            ConfigChange::SetRentCollector {
                collector: new_rent_collector.map(|p| p.to_base58()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use borsh::BorshSerialize;

    fn make_instruction_data(actions: Vec<RawConfigAction>, memo: Option<String>) -> Vec<u8> {
        let args = ConfigTransactionCreateArgs { actions, memo };
        let mut data = vec![0u8; 8];
        args.serialize(&mut data).unwrap();
        data
    }

    fn make_args_data<T: BorshSerialize>(args: &T) -> Vec<u8> {
        let mut data = vec![0u8; 8];
        args.serialize(&mut data).unwrap();
        data
    }

    #[test]
    fn decode_add_member() {
        let member_key = [42u8; 32];
        let actions = vec![RawConfigAction::AddMember {
            new_member: RawMember {
                key: RawPubkey(member_key),
                permissions: 0x03,
            },
        }];

        let data = make_instruction_data(actions, None);
        let changes = decode_config_actions(&data).unwrap();

        assert_eq!(changes.len(), 1);
        match &changes[0] {
            ConfigChange::AddMember {
                member,
                permissions,
            } => {
                assert_eq!(member, &bs58::encode(member_key).into_string());
                assert_eq!(permissions.mask, 0x03);
            }
            other => panic!("expected AddMember, got {other:?}"),
        }
    }

    #[test]
    fn decode_change_threshold() {
        let actions = vec![RawConfigAction::ChangeThreshold { new_threshold: 3 }];
        let data = make_instruction_data(actions, Some("bump threshold".to_string()));
        let changes = decode_config_actions(&data).unwrap();
        assert_eq!(
            changes[0],
            ConfigChange::ChangeThreshold { new_threshold: 3 }
        );
    }

    #[test]
    fn decode_multiple_actions() {
        let actions = vec![
            RawConfigAction::AddMember {
                new_member: RawMember {
                    key: RawPubkey([1u8; 32]),
                    permissions: 0x07,
                },
            },
            RawConfigAction::ChangeThreshold { new_threshold: 2 },
            RawConfigAction::RemoveMember {
                old_member: RawPubkey([2u8; 32]),
            },
        ];
        let data = make_instruction_data(actions, None);
        let changes = decode_config_actions(&data).unwrap();
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn decode_direct_add_member() {
        let args = MultisigAddMemberArgs {
            new_member: RawMember {
                key: RawPubkey([99u8; 32]),
                permissions: 0x06,
            },
            memo: None,
        };
        let data = make_args_data(&args);
        let entry = decode_multisig_add_member(&data).unwrap();
        assert_eq!(entry.permissions.mask, 0x06);
    }

    #[test]
    fn decode_direct_change_threshold() {
        let args = MultisigChangeThresholdArgs {
            new_threshold: 5,
            memo: Some("bump".to_string()),
        };
        let data = make_args_data(&args);
        assert_eq!(decode_multisig_change_threshold(&data).unwrap(), 5);
    }

    #[test]
    fn decode_direct_set_config_authority() {
        let args = MultisigSetConfigAuthorityArgs {
            config_authority: RawPubkey([77u8; 32]),
            memo: None,
        };
        let data = make_args_data(&args);
        let result = decode_multisig_set_config_authority(&data).unwrap();
        assert_eq!(result, bs58::encode([77u8; 32]).into_string());
    }

    #[test]
    fn decode_too_short_data() {
        assert!(decode_config_actions(&[0u8; 4]).is_err());
    }
}

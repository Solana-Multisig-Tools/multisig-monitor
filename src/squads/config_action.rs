use borsh::BorshDeserialize;
#[cfg(test)]
use borsh::BorshSerialize;

use crate::error::MonitorError;
use crate::squads::types::{ConfigChange, Permissions};

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

/// On-chain Period enum for spending limits.
#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
pub enum RawPeriod {
    OneTime,
    Day,
    Week,
    Month,
}

/// On-chain ConfigAction enum, Borsh-serialized in instruction data.
/// Variant indices match the Squads v4 IDL ordering.
#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
#[allow(dead_code)]
pub enum RawConfigAction {
    AddMember { new_member: RawMember },
    RemoveMember { old_member: RawPubkey },
    ChangeThreshold { new_threshold: u16 },
    SetTimeLock { new_time_lock: u32 },
    AddSpendingLimit {
        create_key: RawPubkey,
        vault_index: u8,
        mint: RawPubkey,
        amount: u64,
        period: RawPeriod,
        members: Vec<RawPubkey>,
        destinations: Vec<RawPubkey>,
    },
    RemoveSpendingLimit { spending_limit: RawPubkey },
    SetRentCollector { new_rent_collector: Option<RawPubkey> },
}

/// Instruction arguments for config_transaction_create.
/// Layout after the 8-byte Anchor discriminator:
///   Vec<ConfigAction> (borsh: 4-byte len + items), then Option<String> memo.
#[derive(Debug, Clone, BorshDeserialize)]
#[cfg_attr(test, derive(BorshSerialize))]
#[allow(dead_code)]
pub struct ConfigTransactionCreateArgs {
    pub actions: Vec<RawConfigAction>,
    pub memo: Option<String>,
}

/// Decodes the config_transaction_create instruction arguments from raw data.
/// `data` should be the full instruction data (including the 8-byte discriminator).
pub fn decode_config_actions(data: &[u8]) -> Result<Vec<ConfigChange>, MonitorError> {
    if data.len() < 8 {
        return Err(MonitorError::InstructionDataTooShort {
            expected: 8,
            actual: data.len(),
        });
    }

    let args_data = &data[8..]; // skip discriminator
    let args = ConfigTransactionCreateArgs::try_from_slice(args_data)
        .map_err(|e| MonitorError::ConfigActionDecode(e.to_string()))?;

    Ok(args
        .actions
        .into_iter()
        .map(raw_to_config_change)
        .collect())
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
        RawConfigAction::SetRentCollector {
            new_rent_collector,
        } => ConfigChange::SetRentCollector {
            collector: new_rent_collector.map(|p| p.to_base58()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use borsh::BorshSerialize;

    /// Helper to build fake instruction data: discriminator + borsh-serialized args.
    fn make_instruction_data(actions: Vec<RawConfigAction>, memo: Option<String>) -> Vec<u8> {
        let args = ConfigTransactionCreateArgs { actions, memo };
        let mut data = vec![0u8; 8]; // fake discriminator
        args.serialize(&mut data).unwrap();
        data
    }

    #[test]
    fn decode_add_member() {
        let member_key = [42u8; 32];
        let actions = vec![RawConfigAction::AddMember {
            new_member: RawMember {
                key: RawPubkey(member_key),
                permissions: 0x03, // Initiate + Vote
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
                assert_eq!(permissions.to_labels(), vec!["Initiate", "Vote"]);
            }
            other => panic!("expected AddMember, got {other:?}"),
        }
    }

    #[test]
    fn decode_change_threshold() {
        let actions = vec![RawConfigAction::ChangeThreshold { new_threshold: 3 }];
        let data = make_instruction_data(actions, Some("bump threshold".to_string()));
        let changes = decode_config_actions(&data).unwrap();

        assert_eq!(changes.len(), 1);
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
    fn decode_too_short_data() {
        let result = decode_config_actions(&[0u8; 4]);
        assert!(result.is_err());
    }
}

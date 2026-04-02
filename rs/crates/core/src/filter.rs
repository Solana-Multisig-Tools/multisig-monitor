use crate::error::MonitorError;
use crate::squads::discriminator::InstructionKind;
use crate::squads::types::{ConfigChange, SquadsOperation};

/// Controls which operations produce notifications.
///
/// - `instruction_types`: if empty, all instruction types are monitored.
///   If non-empty, only the listed types produce notifications.
/// - `config_types`: if empty, all config action types are included.
///   If non-empty, only the listed config actions are included.
///   Ignored entirely if config transactions are excluded by `instruction_types`.
#[derive(Debug, Clone, Default)]
pub struct NotificationFilter {
    pub instruction_types: Vec<String>,
    pub config_types: Vec<String>,
}

impl NotificationFilter {
    /// Whether a given instruction kind passes the instruction_types filter.
    pub fn allows_instruction(&self, kind: InstructionKind) -> bool {
        if self.instruction_types.is_empty() {
            return true;
        }
        let key = instruction_kind_key(kind);
        self.instruction_types.iter().any(|s| s == key)
    }

    /// Filters a SquadsOperation, returning None if it should be suppressed.
    /// For ConfigTransactionCreate, also filters individual config actions.
    pub fn apply(&self, operation: SquadsOperation) -> Option<SquadsOperation> {
        match operation {
            SquadsOperation::ConfigTransactionCreate { actions } if !actions.is_empty() => {
                let filtered = self.filter_config_actions(actions);
                if filtered.is_empty() {
                    None
                } else {
                    Some(SquadsOperation::ConfigTransactionCreate { actions: filtered })
                }
            }
            other => Some(other),
        }
    }

    fn filter_config_actions(&self, actions: Vec<ConfigChange>) -> Vec<ConfigChange> {
        if self.config_types.is_empty() {
            return actions;
        }
        actions
            .into_iter()
            .filter(|a| {
                let key = config_change_key(a);
                self.config_types.iter().any(|s| s == key)
            })
            .collect()
    }

    /// Returns all valid instruction type strings for error reporting.
    pub fn valid_instruction_types() -> &'static [&'static str] {
        INSTRUCTION_TYPE_KEYS
    }

    /// Returns all valid config type strings for error reporting.
    pub fn valid_config_types() -> &'static [&'static str] {
        CONFIG_TYPE_KEYS
    }

    /// Validates that all configured strings are recognized.
    pub fn validate(&self) -> Result<(), MonitorError> {
        for s in &self.instruction_types {
            if !INSTRUCTION_TYPE_KEYS.contains(&s.as_str()) {
                return Err(MonitorError::Config(format!(
                    "unknown instruction type '{}'. valid types: {}",
                    s,
                    INSTRUCTION_TYPE_KEYS.join(", ")
                )));
            }
        }
        for s in &self.config_types {
            if !CONFIG_TYPE_KEYS.contains(&s.as_str()) {
                return Err(MonitorError::Config(format!(
                    "unknown config type '{}'. valid types: {}",
                    s,
                    CONFIG_TYPE_KEYS.join(", ")
                )));
            }
        }
        Ok(())
    }
}

const INSTRUCTION_TYPE_KEYS: &[&str] = &[
    "multisig_create",
    "multisig_create_v2",
    "proposal_create",
    "proposal_activate",
    "proposal_approve",
    "proposal_reject",
    "proposal_cancel",
    "proposal_cancel_v2",
    "vault_transaction_create",
    "config_transaction_create",
    "vault_transaction_execute",
    "config_transaction_execute",
    "multisig_add_member",
    "multisig_remove_member",
    "multisig_change_threshold",
    "multisig_set_time_lock",
    "multisig_set_config_authority",
    "multisig_set_rent_collector",
    "multisig_add_spending_limit",
    "multisig_remove_spending_limit",
    "spending_limit_use",
    "transaction_buffer_create",
    "transaction_buffer_extend",
    "transaction_buffer_close",
    "vault_transaction_create_from_buffer",
    "batch_create",
    "batch_add_transaction",
    "batch_execute_transaction",
    "config_transaction_accounts_close",
    "vault_transaction_accounts_close",
    "vault_batch_transaction_account_close",
    "batch_accounts_close",
];

const CONFIG_TYPE_KEYS: &[&str] = &[
    "add_member",
    "remove_member",
    "change_threshold",
    "set_time_lock",
    "add_spending_limit",
    "remove_spending_limit",
    "set_rent_collector",
];

/// Maps an InstructionKind to its config string key.
/// Uses the Anchor instruction name directly as the filter key.
fn instruction_kind_key(kind: InstructionKind) -> &'static str {
    kind.anchor_name()
}

/// Maps a ConfigChange variant to its config string key.
fn config_change_key(change: &ConfigChange) -> &'static str {
    match change {
        ConfigChange::AddMember { .. } => "add_member",
        ConfigChange::RemoveMember { .. } => "remove_member",
        ConfigChange::ChangeThreshold { .. } => "change_threshold",
        ConfigChange::SetTimeLock { .. } => "set_time_lock",
        ConfigChange::AddSpendingLimit { .. } => "add_spending_limit",
        ConfigChange::RemoveSpendingLimit { .. } => "remove_spending_limit",
        ConfigChange::SetRentCollector { .. } => "set_rent_collector",
    }
}

/// Parses a comma-separated string into a list of trimmed, non-empty strings.
pub fn parse_comma_list(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::squads::types::Permissions;

    fn default_filter() -> NotificationFilter {
        NotificationFilter::default()
    }

    // --- instruction_types filter ---

    #[test]
    fn empty_instruction_types_allows_all() {
        let f = default_filter();
        assert!(f.allows_instruction(InstructionKind::ProposalApprove));
        assert!(f.allows_instruction(InstructionKind::ConfigTransactionCreate));
        assert!(f.allows_instruction(InstructionKind::VaultTransactionExecute));
    }

    #[test]
    fn explicit_instruction_types_filters() {
        let f = NotificationFilter {
            instruction_types: vec![
                "proposal_approve".to_string(),
                "proposal_reject".to_string(),
            ],
            ..Default::default()
        };
        assert!(f.allows_instruction(InstructionKind::ProposalApprove));
        assert!(f.allows_instruction(InstructionKind::ProposalReject));
        assert!(!f.allows_instruction(InstructionKind::ProposalCreate));
        assert!(!f.allows_instruction(InstructionKind::ConfigTransactionCreate));
    }

    // --- config_types filter ---

    #[test]
    fn empty_config_types_passes_all_actions() {
        let f = default_filter();
        let actions = vec![
            ConfigChange::AddMember {
                member: "a".into(),
                permissions: Permissions { mask: 7 },
            },
            ConfigChange::ChangeThreshold { new_threshold: 2 },
        ];
        let op = SquadsOperation::ConfigTransactionCreate { actions };
        let result = f.apply(op);
        match result {
            Some(SquadsOperation::ConfigTransactionCreate { actions }) => {
                assert_eq!(actions.len(), 2);
            }
            other => panic!("expected ConfigTransactionCreate, got {other:?}"),
        }
    }

    #[test]
    fn explicit_config_types_filters_actions() {
        let f = NotificationFilter {
            config_types: vec!["add_member".to_string()],
            ..Default::default()
        };
        let actions = vec![
            ConfigChange::AddMember {
                member: "a".into(),
                permissions: Permissions { mask: 7 },
            },
            ConfigChange::ChangeThreshold { new_threshold: 2 },
            ConfigChange::RemoveMember { member: "b".into() },
        ];
        let op = SquadsOperation::ConfigTransactionCreate { actions };
        let result = f.apply(op);
        match result {
            Some(SquadsOperation::ConfigTransactionCreate { actions }) => {
                assert_eq!(actions.len(), 1);
                assert!(matches!(actions[0], ConfigChange::AddMember { .. }));
            }
            other => panic!("expected ConfigTransactionCreate with 1 action, got {other:?}"),
        }
    }

    #[test]
    fn config_types_filter_suppresses_when_all_filtered() {
        let f = NotificationFilter {
            config_types: vec!["set_time_lock".to_string()],
            ..Default::default()
        };
        let actions = vec![
            ConfigChange::AddMember {
                member: "a".into(),
                permissions: Permissions { mask: 7 },
            },
            ConfigChange::ChangeThreshold { new_threshold: 2 },
        ];
        let op = SquadsOperation::ConfigTransactionCreate { actions };
        assert!(
            f.apply(op).is_none(),
            "should suppress when no actions match"
        );
    }

    #[test]
    fn non_config_operations_pass_through_unmodified() {
        let f = NotificationFilter {
            config_types: vec!["add_member".to_string()],
            ..Default::default()
        };
        assert_eq!(
            f.apply(SquadsOperation::ProposalApprove),
            Some(SquadsOperation::ProposalApprove)
        );
    }

    // --- validation ---

    #[test]
    fn validate_accepts_valid_types() {
        let f = NotificationFilter {
            instruction_types: vec![
                "proposal_approve".to_string(),
                "config_transaction_create".to_string(),
            ],
            config_types: vec!["add_member".to_string(), "change_threshold".to_string()],
        };
        assert!(f.validate().is_ok());
    }

    #[test]
    fn validate_rejects_unknown_instruction_type() {
        let f = NotificationFilter {
            instruction_types: vec!["proposal_approve".to_string(), "not_real".to_string()],
            ..Default::default()
        };
        let err = f.validate().unwrap_err();
        assert!(err.to_string().contains("not_real"));
    }

    #[test]
    fn validate_rejects_unknown_config_type() {
        let f = NotificationFilter {
            config_types: vec!["add_member".to_string(), "bogus".to_string()],
            ..Default::default()
        };
        let err = f.validate().unwrap_err();
        assert!(err.to_string().contains("bogus"));
    }

    // --- parse_comma_list ---

    #[test]
    fn parse_comma_list_basic() {
        assert_eq!(
            parse_comma_list("proposal_approve, proposal_reject"),
            vec!["proposal_approve", "proposal_reject"]
        );
    }

    #[test]
    fn parse_comma_list_empty_string() {
        assert!(parse_comma_list("").is_empty());
    }

    #[test]
    fn parse_comma_list_trims_whitespace() {
        assert_eq!(
            parse_comma_list("  add_member , change_threshold  "),
            vec!["add_member", "change_threshold"]
        );
    }
}

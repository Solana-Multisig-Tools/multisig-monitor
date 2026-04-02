/// The Squads v4 program ID on mainnet.
pub const SQUADS_PROGRAM_ID: &str = "SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf";

/// A detected Squads v4 operation extracted from a raw transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedOperation {
    pub signature: String,
    pub multisig: String,
    pub member: String,
    pub operation: SquadsOperation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SquadsOperation {
    ProposalCreate,
    ProposalActivate,
    ProposalApprove,
    ProposalReject,
    ProposalCancel,
    VaultTransactionCreate,
    ConfigTransactionCreate { actions: Vec<ConfigChange> },
    VaultTransactionExecute,
    ConfigTransactionExecute,
}

impl SquadsOperation {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ProposalCreate => "Proposal Created",
            Self::ProposalActivate => "Proposal Activated",
            Self::ProposalApprove => "Proposal Approved",
            Self::ProposalReject => "Proposal Rejected",
            Self::ProposalCancel => "Proposal Cancelled",
            Self::VaultTransactionCreate => "Vault Transaction Created",
            Self::ConfigTransactionCreate { .. } => "Config Transaction Created",
            Self::VaultTransactionExecute => "Vault Transaction Executed",
            Self::ConfigTransactionExecute => "Config Transaction Executed",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Self::ProposalCreate => "\u{1f4dd}",
            Self::ProposalActivate => "\u{26a1}",
            Self::ProposalApprove => "\u{2705}",
            Self::ProposalReject => "\u{274c}",
            Self::ProposalCancel => "\u{1f6ab}",
            Self::VaultTransactionCreate => "\u{1f4e6}",
            Self::ConfigTransactionCreate { .. } => "\u{2699}\u{fe0f}",
            Self::VaultTransactionExecute => "\u{1f680}",
            Self::ConfigTransactionExecute => "\u{1f680}",
        }
    }

    pub fn member_role(&self) -> &'static str {
        match self {
            Self::ProposalCreate | Self::VaultTransactionCreate => "Creator",
            Self::ConfigTransactionCreate { .. } => "Creator",
            Self::ProposalActivate => "Activator",
            Self::ProposalApprove => "Approver",
            Self::ProposalReject => "Rejector",
            Self::ProposalCancel => "Canceller",
            Self::VaultTransactionExecute | Self::ConfigTransactionExecute => "Executor",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigChange {
    AddMember {
        member: String,
        permissions: Permissions,
    },
    RemoveMember {
        member: String,
    },
    ChangeThreshold {
        new_threshold: u16,
    },
    SetTimeLock {
        seconds: u32,
    },
    AddSpendingLimit {
        create_key: String,
        vault_index: u8,
        mint: String,
        amount: u64,
        members: Vec<String>,
        destinations: Vec<String>,
    },
    RemoveSpendingLimit {
        spending_limit: String,
    },
    SetRentCollector {
        collector: Option<String>,
    },
}

impl ConfigChange {
    pub fn description(&self) -> String {
        match self {
            Self::AddMember {
                member,
                permissions,
            } => {
                let perms = permissions.to_labels().join(", ");
                format!("Add member {member} (permissions: {perms})")
            }
            Self::RemoveMember { member } => format!("Remove member {member}"),
            Self::ChangeThreshold { new_threshold } => {
                format!("Change threshold to {new_threshold}")
            }
            Self::SetTimeLock { seconds } => format!("Set time lock to {seconds}s"),
            Self::AddSpendingLimit {
                mint,
                amount,
                vault_index,
                ..
            } => format!("Add spending limit: {amount} of {mint} on vault {vault_index}"),
            Self::RemoveSpendingLimit { spending_limit } => {
                format!("Remove spending limit {spending_limit}")
            }
            Self::SetRentCollector { collector } => match collector {
                Some(addr) => format!("Set rent collector to {addr}"),
                None => "Remove rent collector".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Permissions {
    pub mask: u8,
}

impl Permissions {
    pub const INITIATE: u8 = 1;
    pub const VOTE: u8 = 2;
    pub const EXECUTE: u8 = 4;

    pub fn to_labels(&self) -> Vec<&'static str> {
        let mut labels = Vec::new();
        if self.mask & Self::INITIATE != 0 {
            labels.push("Initiate");
        }
        if self.mask & Self::VOTE != 0 {
            labels.push("Vote");
        }
        if self.mask & Self::EXECUTE != 0 {
            labels.push("Execute");
        }
        if labels.is_empty() {
            labels.push("None");
        }
        labels
    }
}

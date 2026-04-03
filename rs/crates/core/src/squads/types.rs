/// The Squads v4 program ID on mainnet.
pub const SQUADS_PROGRAM_ID: &str = "SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf";

/// A detected Squads v4 operation extracted from a raw transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedOperation {
    pub signature: String,
    pub multisig: String,
    pub member: String,
    /// The on-chain transaction account address, if the instruction references one.
    /// Useful for correlating creates with proposals and executions.
    pub transaction_address: Option<String>,
    /// The on-chain proposal account address, if the instruction references one.
    pub proposal_address: Option<String>,
    pub operation: SquadsOperation,
    pub block_time: Option<i64>,
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum SquadsOperation {
    // -- Multisig creation --
    MultisigCreate {
        config_authority: Option<String>,
        threshold: u16,
        members: Vec<MemberEntry>,
    },

    // -- Proposal lifecycle --
    ProposalCreate {
        draft: bool,
    },
    ProposalActivate,
    ProposalApprove,
    ProposalReject,
    ProposalCancel,
    ProposalCancelV2,

    // -- Transaction creation --
    VaultTransactionCreate,
    ConfigTransactionCreate {
        actions: Vec<ConfigChange>,
    },

    // -- Transaction execution --
    VaultTransactionExecute,
    ConfigTransactionExecute,

    // -- Controlled multisig direct config (bypasses proposals) --
    DirectAddMember {
        new_member: MemberEntry,
    },
    DirectRemoveMember {
        old_member: String,
    },
    DirectChangeThreshold {
        new_threshold: u16,
    },
    DirectSetTimeLock {
        time_lock: u32,
    },
    DirectSetConfigAuthority {
        config_authority: String,
    },
    DirectSetRentCollector {
        rent_collector: Option<String>,
    },
    DirectAddSpendingLimit {
        create_key: String,
        vault_index: u8,
        mint: String,
        amount: u64,
        members: Vec<String>,
        destinations: Vec<String>,
    },
    DirectRemoveSpendingLimit,

    // -- Spending limit usage --
    SpendingLimitUse {
        amount: u64,
        decimals: u8,
    },

    // -- Buffer-based transaction creation --
    TransactionBufferCreate,
    TransactionBufferExtend,
    TransactionBufferClose,
    VaultTransactionCreateFromBuffer,

    // -- Batch operations --
    BatchCreate,
    BatchAddTransaction,
    BatchExecuteTransaction,

    // -- Account cleanup --
    ConfigTransactionAccountsClose,
    VaultTransactionAccountsClose,
    VaultBatchTransactionAccountClose,
    BatchAccountsClose,
}

impl SquadsOperation {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::MultisigCreate { .. } => "Multisig Created",
            Self::ProposalCreate { .. } => "Proposal Created",
            Self::ProposalActivate => "Proposal Activated",
            Self::ProposalApprove => "Proposal Approved",
            Self::ProposalReject => "Proposal Rejected",
            Self::ProposalCancel => "Proposal Cancelled",
            Self::ProposalCancelV2 => "Proposal Cancelled (V2)",
            Self::VaultTransactionCreate => "Vault Transaction Created",
            Self::ConfigTransactionCreate { .. } => "Config Transaction Created",
            Self::VaultTransactionExecute => "Vault Transaction Executed",
            Self::ConfigTransactionExecute => "Config Transaction Executed",
            Self::DirectAddMember { .. } => "Direct: Member Added",
            Self::DirectRemoveMember { .. } => "Direct: Member Removed",
            Self::DirectChangeThreshold { .. } => "Direct: Threshold Changed",
            Self::DirectSetTimeLock { .. } => "Direct: Time Lock Set",
            Self::DirectSetConfigAuthority { .. } => "Direct: Config Authority Changed",
            Self::DirectSetRentCollector { .. } => "Direct: Rent Collector Set",
            Self::DirectAddSpendingLimit { .. } => "Direct: Spending Limit Added",
            Self::DirectRemoveSpendingLimit => "Direct: Spending Limit Removed",
            Self::SpendingLimitUse { .. } => "Spending Limit Used",
            Self::TransactionBufferCreate => "Transaction Buffer Created",
            Self::TransactionBufferExtend => "Transaction Buffer Extended",
            Self::TransactionBufferClose => "Transaction Buffer Closed",
            Self::VaultTransactionCreateFromBuffer => "Vault Transaction Created (from Buffer)",
            Self::BatchCreate => "Batch Created",
            Self::BatchAddTransaction => "Batch Transaction Added",
            Self::BatchExecuteTransaction => "Batch Transaction Executed",
            Self::ConfigTransactionAccountsClose => "Config Transaction Accounts Closed",
            Self::VaultTransactionAccountsClose => "Vault Transaction Accounts Closed",
            Self::VaultBatchTransactionAccountClose => "Vault Batch Transaction Account Closed",
            Self::BatchAccountsClose => "Batch Accounts Closed",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Self::MultisigCreate { .. } => "\u{1f3d7}\u{fe0f}",
            Self::ProposalCreate { .. } => "\u{1f4dd}",
            Self::ProposalActivate => "\u{26a1}",
            Self::ProposalApprove => "\u{2705}",
            Self::ProposalReject => "\u{274c}",
            Self::ProposalCancel | Self::ProposalCancelV2 => "\u{1f6ab}",
            Self::VaultTransactionCreate | Self::VaultTransactionCreateFromBuffer => "\u{1f4e6}",
            Self::ConfigTransactionCreate { .. } => "\u{2699}\u{fe0f}",
            Self::VaultTransactionExecute | Self::ConfigTransactionExecute => "\u{1f680}",
            Self::DirectAddMember { .. }
            | Self::DirectRemoveMember { .. }
            | Self::DirectChangeThreshold { .. }
            | Self::DirectSetTimeLock { .. }
            | Self::DirectSetConfigAuthority { .. }
            | Self::DirectSetRentCollector { .. }
            | Self::DirectAddSpendingLimit { .. }
            | Self::DirectRemoveSpendingLimit => "\u{26a0}\u{fe0f}",
            Self::SpendingLimitUse { .. } => "\u{1f4b8}",
            Self::TransactionBufferCreate
            | Self::TransactionBufferExtend
            | Self::TransactionBufferClose => "\u{1f4cb}",
            Self::BatchCreate | Self::BatchAddTransaction | Self::BatchExecuteTransaction => {
                "\u{1f4e6}"
            }
            Self::ConfigTransactionAccountsClose
            | Self::VaultTransactionAccountsClose
            | Self::VaultBatchTransactionAccountClose
            | Self::BatchAccountsClose => "\u{1f9f9}",
        }
    }

    pub fn member_role(&self) -> &'static str {
        match self {
            Self::MultisigCreate { .. } => "Creator",
            Self::ProposalCreate { .. }
            | Self::VaultTransactionCreate
            | Self::VaultTransactionCreateFromBuffer
            | Self::ConfigTransactionCreate { .. }
            | Self::TransactionBufferCreate
            | Self::TransactionBufferExtend
            | Self::TransactionBufferClose
            | Self::BatchCreate => "Creator",
            Self::ProposalActivate => "Activator",
            Self::ProposalApprove => "Approver",
            Self::ProposalReject => "Rejector",
            Self::ProposalCancel | Self::ProposalCancelV2 => "Canceller",
            Self::VaultTransactionExecute
            | Self::ConfigTransactionExecute
            | Self::BatchExecuteTransaction => "Executor",
            Self::BatchAddTransaction => "Member",
            Self::DirectAddMember { .. }
            | Self::DirectRemoveMember { .. }
            | Self::DirectChangeThreshold { .. }
            | Self::DirectSetTimeLock { .. }
            | Self::DirectSetConfigAuthority { .. }
            | Self::DirectSetRentCollector { .. }
            | Self::DirectAddSpendingLimit { .. }
            | Self::DirectRemoveSpendingLimit => "Config Authority",
            Self::SpendingLimitUse { .. } => "Member",
            Self::ConfigTransactionAccountsClose
            | Self::VaultTransactionAccountsClose
            | Self::VaultBatchTransactionAccountClose
            | Self::BatchAccountsClose => "Closer",
        }
    }
}

// ---------------------------------------------------------------------------
// Config changes (used in config_transaction_create args)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct MemberEntry {
    pub key: String,
    pub permissions: Permissions,
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

pub fn truncate_address(address: &str) -> String {
    if address.len() <= 12 {
        return address.to_string();
    }
    format!("{}...{}", &address[..4], &address[address.len() - 4..])
}

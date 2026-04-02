use serde::Deserialize;

/// Top-level transaction from a Helius raw webhook POST body.
/// Helius sends an array of these.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTransaction {
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub transaction: TransactionEnvelope,
    pub meta: Option<TransactionMeta>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransactionEnvelope {
    pub message: TransactionMessage,
    pub signatures: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionMessage {
    pub account_keys: Vec<String>,
    pub instructions: Vec<CompiledInstruction>,
}

/// A compiled instruction as it appears in a raw Solana transaction.
/// `accounts` are indices into the top-level `account_keys` array.
/// `data` is base58-encoded instruction data.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledInstruction {
    pub program_id_index: u8,
    pub accounts: Vec<u8>,
    pub data: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionMeta {
    pub err: Option<serde_json::Value>,
    pub inner_instructions: Option<Vec<InnerInstructionSet>>,
    pub log_messages: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InnerInstructionSet {
    pub index: u8,
    pub instructions: Vec<CompiledInstruction>,
}

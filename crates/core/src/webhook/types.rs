use serde::Deserialize;

/// Top-level transaction from a Helius raw webhook POST body.
/// Helius sends an array of these, following the standard Solana
/// getTransaction response format. The transaction signature is
/// inside `transaction.signatures[0]`, not at the top level.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTransaction {
    /// Present in some Helius payload variants but not guaranteed in raw webhooks.
    /// Falls back to `transaction.signatures[0]` via `RawTransaction::signature()`.
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub slot: u64,
    pub block_time: Option<i64>,
    #[serde(default)]
    pub index_within_block: Option<u64>,
    pub transaction: TransactionEnvelope,
    pub meta: Option<TransactionMeta>,
}

impl RawTransaction {
    /// Returns the transaction signature, preferring the top-level field
    /// and falling back to `transaction.signatures[0]`.
    pub fn signature(&self) -> Option<&str> {
        self.signature
            .as_deref()
            .or_else(|| self.transaction.signatures.first().map(|s| s.as_str()))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransactionEnvelope {
    pub message: TransactionMessage,
    #[serde(default)]
    pub signatures: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionMessage {
    pub account_keys: Vec<AccountKeyEntry>,
    pub instructions: Vec<CompiledInstruction>,
}

/// Account keys can appear as either plain strings or objects with
/// `pubkey`/`signer`/`writable` fields depending on encoding.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AccountKeyEntry {
    /// Plain base58 pubkey string (standard encoding)
    Plain(String),
    /// Structured entry with metadata (jsonParsed encoding)
    Structured {
        pubkey: String,
        #[serde(default)]
        signer: bool,
        #[serde(default)]
        writable: bool,
    },
}

impl AccountKeyEntry {
    pub fn pubkey(&self) -> &str {
        match self {
            Self::Plain(s) => s,
            Self::Structured { pubkey, .. } => pubkey,
        }
    }
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

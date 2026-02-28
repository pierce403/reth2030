//! Execution-layer primitive types for `reth2030`.
//!
//! This crate intentionally keeps a compact, well-documented surface for
//! block/transaction/receipt representations that can be shared by core,
//! RPC, and networking crates.

use serde::{Deserialize, Serialize};

pub type Address = [u8; 20];
pub type Hash32 = [u8; 32];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegacyTx {
    pub nonce: u64,
    pub from: Address,
    pub to: Option<Address>,
    pub gas_limit: u64,
    #[serde(with = "u128_string")]
    pub gas_price: u128,
    #[serde(with = "u128_string")]
    pub value: u128,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Eip1559Tx {
    pub nonce: u64,
    pub from: Address,
    pub to: Option<Address>,
    pub gas_limit: u64,
    #[serde(with = "u128_string")]
    pub max_fee_per_gas: u128,
    #[serde(with = "u128_string")]
    pub max_priority_fee_per_gas: u128,
    #[serde(with = "u128_string")]
    pub value: u128,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlobTx {
    pub nonce: u64,
    pub from: Address,
    pub to: Option<Address>,
    pub gas_limit: u64,
    #[serde(with = "u128_string")]
    pub max_fee_per_gas: u128,
    #[serde(with = "u128_string")]
    pub max_priority_fee_per_gas: u128,
    #[serde(with = "u128_string")]
    pub max_fee_per_blob_gas: u128,
    #[serde(with = "u128_string")]
    pub value: u128,
    pub data: Vec<u8>,
    pub blob_versioned_hashes: Vec<Hash32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "tx_type", rename_all = "snake_case")]
pub enum Transaction {
    Legacy(LegacyTx),
    Eip1559(Eip1559Tx),
    Blob(BlobTx),
}

impl Transaction {
    pub fn nonce(&self) -> u64 {
        match self {
            Self::Legacy(tx) => tx.nonce,
            Self::Eip1559(tx) => tx.nonce,
            Self::Blob(tx) => tx.nonce,
        }
    }

    pub fn from(&self) -> Address {
        match self {
            Self::Legacy(tx) => tx.from,
            Self::Eip1559(tx) => tx.from,
            Self::Blob(tx) => tx.from,
        }
    }

    pub fn to(&self) -> Option<Address> {
        match self {
            Self::Legacy(tx) => tx.to,
            Self::Eip1559(tx) => tx.to,
            Self::Blob(tx) => tx.to,
        }
    }

    pub fn gas_limit(&self) -> u64 {
        match self {
            Self::Legacy(tx) => tx.gas_limit,
            Self::Eip1559(tx) => tx.gas_limit,
            Self::Blob(tx) => tx.gas_limit,
        }
    }

    pub fn value(&self) -> u128 {
        match self {
            Self::Legacy(tx) => tx.value,
            Self::Eip1559(tx) => tx.value,
            Self::Blob(tx) => tx.value,
        }
    }

    pub fn payload(&self) -> &[u8] {
        match self {
            Self::Legacy(tx) => tx.data.as_slice(),
            Self::Eip1559(tx) => tx.data.as_slice(),
            Self::Blob(tx) => tx.data.as_slice(),
        }
    }

    /// JSON boundary used for simple persistence and test fixtures.
    pub fn to_json_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// JSON boundary used for simple persistence and test fixtures.
    pub fn from_json_bytes(input: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(input)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogEntry {
    pub address: Address,
    pub topics: Vec<Hash32>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Receipt {
    pub tx_hash: Hash32,
    pub success: bool,
    pub cumulative_gas_used: u64,
    pub logs: Vec<LogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Header {
    pub parent_hash: Hash32,
    pub number: u64,
    pub timestamp: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub state_root: Hash32,
    pub transactions_root: Hash32,
    pub receipts_root: Hash32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    GasUsedExceedsLimit,
    ReceiptCountMismatch,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::GasUsedExceedsLimit => {
                write!(f, "header.gas_used must be <= header.gas_limit")
            }
            ValidationError::ReceiptCountMismatch => {
                write!(
                    f,
                    "block.receipts length must match block.transactions length"
                )
            }
        }
    }
}

impl std::error::Error for ValidationError {}

impl Header {
    pub fn validate_basic(&self) -> Result<(), ValidationError> {
        if self.gas_used > self.gas_limit {
            return Err(ValidationError::GasUsedExceedsLimit);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block {
    pub header: Header,
    pub transactions: Vec<Transaction>,
    pub receipts: Vec<Receipt>,
}

impl Block {
    pub fn validate_basic(&self) -> Result<(), ValidationError> {
        self.header.validate_basic()?;
        if self.receipts.len() != self.transactions.len() {
            return Err(ValidationError::ReceiptCountMismatch);
        }
        Ok(())
    }

    /// JSON boundary used for fixture and replay workflows.
    pub fn to_json_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// JSON boundary used for fixture and replay workflows.
    pub fn from_json_bytes(input: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(input)
    }
}

mod u128_string {
    use serde::{de::Error, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(s) => s
                .parse::<u128>()
                .map_err(|_| D::Error::custom("invalid u128 string")),
            serde_json::Value::Number(n) => {
                let as_u64 = n
                    .as_u64()
                    .ok_or_else(|| D::Error::custom("u128 must be a non-negative integer"))?;
                Ok(u128::from(as_u64))
            }
            _ => Err(D::Error::custom(
                "u128 must be encoded as string or integer",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Address, Block, Eip1559Tx, Header, LegacyTx, LogEntry, Receipt, Transaction,
        ValidationError,
    };

    fn addr(byte: u8) -> Address {
        [byte; 20]
    }

    fn sample_header() -> Header {
        Header {
            parent_hash: [0; 32],
            number: 1,
            timestamp: 1_762_312_000,
            gas_limit: 30_000_000,
            gas_used: 42_000,
            state_root: [1; 32],
            transactions_root: [2; 32],
            receipts_root: [3; 32],
        }
    }

    #[test]
    fn transaction_json_roundtrip() {
        let tx = Transaction::Eip1559(Eip1559Tx {
            nonce: 7,
            from: addr(0x11),
            to: Some(addr(0x22)),
            gas_limit: 21_000,
            max_fee_per_gas: 10,
            max_priority_fee_per_gas: 1,
            value: 123,
            data: vec![0xde, 0xad, 0xbe, 0xef],
        });

        let encoded = tx.to_json_bytes().expect("tx serialization");
        let decoded = Transaction::from_json_bytes(&encoded).expect("tx deserialization");
        assert_eq!(decoded, tx);
    }

    #[test]
    fn block_validate_detects_receipt_length_mismatch() {
        let tx = Transaction::Legacy(LegacyTx {
            nonce: 1,
            from: addr(0x01),
            to: Some(addr(0x02)),
            gas_limit: 21_000,
            gas_price: 5,
            value: 1,
            data: Vec::new(),
        });

        let block = Block {
            header: sample_header(),
            transactions: vec![tx],
            receipts: Vec::new(),
        };

        assert_eq!(
            block.validate_basic(),
            Err(ValidationError::ReceiptCountMismatch)
        );
    }

    #[test]
    fn block_json_roundtrip() {
        let tx = Transaction::Legacy(LegacyTx {
            nonce: 1,
            from: addr(0x01),
            to: Some(addr(0x02)),
            gas_limit: 21_000,
            gas_price: 5,
            value: 1,
            data: vec![0xab],
        });

        let receipt = Receipt {
            tx_hash: [9; 32],
            success: true,
            cumulative_gas_used: 21_000,
            logs: vec![LogEntry {
                address: addr(0x44),
                topics: vec![[7; 32]],
                data: vec![1, 2, 3],
            }],
        };

        let block = Block {
            header: sample_header(),
            transactions: vec![tx],
            receipts: vec![receipt],
        };

        let encoded = block.to_json_bytes().expect("block serialization");
        let decoded = Block::from_json_bytes(&encoded).expect("block deserialization");
        assert_eq!(decoded, block);
    }
}

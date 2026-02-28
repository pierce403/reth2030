//! Execution-layer primitive types for `reth2030`.
//!
//! This crate intentionally keeps a compact, well-documented surface for
//! block/transaction/receipt representations that can be shared by core,
//! RPC, and networking crates.
//!
//! ## Public API
//! - `Address`: canonical 20-byte account address type.
//! - `Hash32`: canonical 32-byte hash type.
//! - `LegacyTx`: legacy transaction payload.
//! - `Eip1559Tx`: EIP-1559 transaction payload.
//! - `BlobTx`: blob-carrying transaction payload.
//! - `Transaction`: tagged transaction enum covering supported variants.
//! - `LogEntry`: receipt log payload.
//! - `Receipt`: post-execution transaction receipt payload.
//! - `Header`: block header payload and basic validation helpers.
//! - `ValidationError`: block/header validation error surface.
//! - `Block`: block payload with transaction and receipt lists.

use serde::{Deserialize, Serialize};

pub type Address = [u8; 20];
pub type Hash32 = [u8; 32];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct LogEntry {
    pub address: Address,
    pub topics: Vec<Hash32>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub tx_hash: Hash32,
    pub success: bool,
    pub cumulative_gas_used: u64,
    pub logs: Vec<LogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct Block {
    pub header: Header,
    pub transactions: Vec<Transaction>,
    pub receipts: Vec<Receipt>,
}

impl Block {
    pub fn validate_basic(&self) -> Result<(), ValidationError> {
        self.header.validate_basic()?;
        if !self.receipts.is_empty() && self.receipts.len() != self.transactions.len() {
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
        Address, BlobTx, Block, Eip1559Tx, Header, LegacyTx, LogEntry, Receipt, Transaction,
        ValidationError,
    };
    use serde_json::json;

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
    fn blob_transaction_json_roundtrip() {
        let tx = Transaction::Blob(BlobTx {
            nonce: 9,
            from: addr(0x33),
            to: Some(addr(0x44)),
            gas_limit: 200_000,
            max_fee_per_gas: 100,
            max_priority_fee_per_gas: 2,
            max_fee_per_blob_gas: 12,
            value: 77,
            data: vec![1, 2, 3, 4],
            blob_versioned_hashes: vec![[0xaa; 32], [0xbb; 32]],
        });

        let encoded = tx.to_json_bytes().expect("tx serialization");
        let decoded = Transaction::from_json_bytes(&encoded).expect("tx deserialization");
        assert_eq!(decoded, tx);
    }

    #[test]
    fn transaction_accessors_cover_all_variants() {
        let legacy = Transaction::Legacy(LegacyTx {
            nonce: 1,
            from: addr(0x01),
            to: Some(addr(0x02)),
            gas_limit: 21_000,
            gas_price: 5,
            value: 6,
            data: vec![0xaa],
        });
        assert_eq!(legacy.nonce(), 1);
        assert_eq!(legacy.from(), addr(0x01));
        assert_eq!(legacy.to(), Some(addr(0x02)));
        assert_eq!(legacy.gas_limit(), 21_000);
        assert_eq!(legacy.value(), 6);
        assert_eq!(legacy.payload(), &[0xaa]);

        let eip1559 = Transaction::Eip1559(Eip1559Tx {
            nonce: 2,
            from: addr(0x03),
            to: None,
            gas_limit: 30_000,
            max_fee_per_gas: 40,
            max_priority_fee_per_gas: 3,
            value: 8,
            data: vec![0xbb, 0xcc],
        });
        assert_eq!(eip1559.nonce(), 2);
        assert_eq!(eip1559.from(), addr(0x03));
        assert_eq!(eip1559.to(), None);
        assert_eq!(eip1559.gas_limit(), 30_000);
        assert_eq!(eip1559.value(), 8);
        assert_eq!(eip1559.payload(), &[0xbb, 0xcc]);

        let blob = Transaction::Blob(BlobTx {
            nonce: 3,
            from: addr(0x04),
            to: Some(addr(0x05)),
            gas_limit: 45_000,
            max_fee_per_gas: 50,
            max_priority_fee_per_gas: 4,
            max_fee_per_blob_gas: 15,
            value: 9,
            data: vec![0xdd],
            blob_versioned_hashes: vec![[0x01; 32]],
        });
        assert_eq!(blob.nonce(), 3);
        assert_eq!(blob.from(), addr(0x04));
        assert_eq!(blob.to(), Some(addr(0x05)));
        assert_eq!(blob.gas_limit(), 45_000);
        assert_eq!(blob.value(), 9);
        assert_eq!(blob.payload(), &[0xdd]);
    }

    #[test]
    fn transaction_u128_fields_serialize_as_strings() {
        let legacy = Transaction::Legacy(LegacyTx {
            nonce: 1,
            from: addr(0x01),
            to: None,
            gas_limit: 21_000,
            gas_price: u128::MAX,
            value: u128::MAX - 1,
            data: Vec::new(),
        });
        let legacy_json: serde_json::Value =
            serde_json::from_slice(&legacy.to_json_bytes().expect("legacy serialization"))
                .expect("legacy json parse");
        assert_eq!(legacy_json["gas_price"], json!(u128::MAX.to_string()));
        assert_eq!(legacy_json["value"], json!((u128::MAX - 1).to_string()));

        let eip1559 = Transaction::Eip1559(Eip1559Tx {
            nonce: 2,
            from: addr(0x02),
            to: Some(addr(0x03)),
            gas_limit: 25_000,
            max_fee_per_gas: u128::MAX,
            max_priority_fee_per_gas: u128::MAX - 2,
            value: u128::MAX - 3,
            data: vec![1],
        });
        let eip1559_json: serde_json::Value =
            serde_json::from_slice(&eip1559.to_json_bytes().expect("eip1559 serialization"))
                .expect("eip1559 json parse");
        assert_eq!(
            eip1559_json["max_fee_per_gas"],
            json!(u128::MAX.to_string())
        );
        assert_eq!(
            eip1559_json["max_priority_fee_per_gas"],
            json!((u128::MAX - 2).to_string())
        );
        assert_eq!(eip1559_json["value"], json!((u128::MAX - 3).to_string()));

        let blob = Transaction::Blob(BlobTx {
            nonce: 3,
            from: addr(0x04),
            to: Some(addr(0x05)),
            gas_limit: 60_000,
            max_fee_per_gas: u128::MAX - 4,
            max_priority_fee_per_gas: u128::MAX - 5,
            max_fee_per_blob_gas: u128::MAX - 6,
            value: u128::MAX - 7,
            data: vec![2],
            blob_versioned_hashes: vec![[0xab; 32]],
        });
        let blob_json: serde_json::Value =
            serde_json::from_slice(&blob.to_json_bytes().expect("blob serialization"))
                .expect("blob json parse");
        assert_eq!(
            blob_json["max_fee_per_gas"],
            json!((u128::MAX - 4).to_string())
        );
        assert_eq!(
            blob_json["max_priority_fee_per_gas"],
            json!((u128::MAX - 5).to_string())
        );
        assert_eq!(
            blob_json["max_fee_per_blob_gas"],
            json!((u128::MAX - 6).to_string())
        );
        assert_eq!(blob_json["value"], json!((u128::MAX - 7).to_string()));
    }

    #[test]
    fn transaction_u128_fields_accept_integer_inputs() {
        let tx: Transaction = serde_json::from_value(json!({
            "tx_type": "legacy",
            "nonce": 11,
            "from": vec![1_u8; 20],
            "to": vec![2_u8; 20],
            "gas_limit": 21_000,
            "gas_price": u64::MAX,
            "value": 9,
            "data": []
        }))
        .expect("legacy with numeric u128 fields should deserialize");

        assert_eq!(
            tx,
            Transaction::Legacy(LegacyTx {
                nonce: 11,
                from: addr(0x01),
                to: Some(addr(0x02)),
                gas_limit: 21_000,
                gas_price: u128::from(u64::MAX),
                value: 9,
                data: Vec::new(),
            })
        );
    }

    #[test]
    fn transaction_u128_fields_reject_invalid_string() {
        let err = serde_json::from_value::<Transaction>(json!({
            "tx_type": "legacy",
            "nonce": 1,
            "from": vec![1_u8; 20],
            "to": null,
            "gas_limit": 21_000,
            "gas_price": "not-a-number",
            "value": "0",
            "data": []
        }))
        .expect_err("invalid u128 string must fail")
        .to_string();
        assert!(
            err.contains("invalid u128 string"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn transaction_u128_fields_reject_negative_integer() {
        let err = serde_json::from_value::<Transaction>(json!({
            "tx_type": "legacy",
            "nonce": 1,
            "from": vec![1_u8; 20],
            "to": null,
            "gas_limit": 21_000,
            "gas_price": -1,
            "value": "0",
            "data": []
        }))
        .expect_err("negative number must fail")
        .to_string();
        assert!(
            err.contains("u128 must be a non-negative integer"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn transaction_u128_fields_reject_non_numeric_types() {
        let err = serde_json::from_value::<Transaction>(json!({
            "tx_type": "legacy",
            "nonce": 1,
            "from": vec![1_u8; 20],
            "to": null,
            "gas_limit": 21_000,
            "gas_price": true,
            "value": "0",
            "data": []
        }))
        .expect_err("invalid type must fail")
        .to_string();
        assert!(
            err.contains("u128 must be encoded as string or integer"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn transaction_deserialization_rejects_unknown_fields() {
        let err = serde_json::from_value::<Transaction>(json!({
            "tx_type": "legacy",
            "nonce": 1,
            "from": vec![1_u8; 20],
            "to": null,
            "gas_limit": 21_000,
            "gas_price": "1",
            "value": "0",
            "data": [],
            "unexpected": 123
        }))
        .expect_err("unknown field must fail")
        .to_string();
        assert!(err.contains("unknown field"), "unexpected error: {err}");
    }

    #[test]
    fn transaction_deserialization_rejects_unknown_tx_type() {
        let err = serde_json::from_value::<Transaction>(json!({
            "tx_type": "future",
            "nonce": 1,
            "from": vec![1_u8; 20],
            "to": null,
            "gas_limit": 21_000,
            "gas_price": "1",
            "value": "0",
            "data": []
        }))
        .expect_err("unknown tx type must fail")
        .to_string();
        assert!(err.contains("unknown variant"), "unexpected error: {err}");
    }

    #[test]
    fn header_validate_accepts_equal_gas_used_and_limit() {
        let mut header = sample_header();
        header.gas_used = header.gas_limit;
        assert_eq!(header.validate_basic(), Ok(()));
    }

    #[test]
    fn header_validate_rejects_excess_gas_used() {
        let mut header = sample_header();
        header.gas_used = header.gas_limit + 1;
        assert_eq!(
            header.validate_basic(),
            Err(ValidationError::GasUsedExceedsLimit)
        );
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
            receipts: vec![
                Receipt {
                    tx_hash: [9; 32],
                    success: true,
                    cumulative_gas_used: 21_000,
                    logs: Vec::new(),
                },
                Receipt {
                    tx_hash: [10; 32],
                    success: true,
                    cumulative_gas_used: 42_000,
                    logs: Vec::new(),
                },
            ],
        };

        assert_eq!(
            block.validate_basic(),
            Err(ValidationError::ReceiptCountMismatch)
        );
    }

    #[test]
    fn block_validate_allows_empty_receipts_for_pre_execution_blocks() {
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

        assert_eq!(block.validate_basic(), Ok(()));
    }

    #[test]
    fn block_validate_propagates_header_error_before_receipt_mismatch() {
        let mut header = sample_header();
        header.gas_limit = 21_000;
        header.gas_used = 21_001;

        let block = Block {
            header,
            transactions: vec![Transaction::Legacy(LegacyTx {
                nonce: 1,
                from: addr(0x01),
                to: Some(addr(0x02)),
                gas_limit: 21_000,
                gas_price: 5,
                value: 1,
                data: Vec::new(),
            })],
            receipts: vec![
                Receipt {
                    tx_hash: [9; 32],
                    success: true,
                    cumulative_gas_used: 21_000,
                    logs: Vec::new(),
                },
                Receipt {
                    tx_hash: [10; 32],
                    success: true,
                    cumulative_gas_used: 42_000,
                    logs: Vec::new(),
                },
            ],
        };

        assert_eq!(
            block.validate_basic(),
            Err(ValidationError::GasUsedExceedsLimit)
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

    #[test]
    fn block_deserialization_rejects_unknown_fields() {
        let block = Block {
            header: sample_header(),
            transactions: vec![Transaction::Legacy(LegacyTx {
                nonce: 1,
                from: addr(0x01),
                to: Some(addr(0x02)),
                gas_limit: 21_000,
                gas_price: 5,
                value: 1,
                data: Vec::new(),
            })],
            receipts: vec![Receipt {
                tx_hash: [9; 32],
                success: true,
                cumulative_gas_used: 21_000,
                logs: Vec::new(),
            }],
        };

        let mut value = serde_json::to_value(block).expect("serialize block");
        value
            .as_object_mut()
            .expect("block must serialize to object")
            .insert("unexpected".to_string(), json!("extra"));

        let err = serde_json::from_value::<Block>(value)
            .expect_err("unknown field must fail")
            .to_string();
        assert!(err.contains("unknown field"), "unexpected error: {err}");
    }
}

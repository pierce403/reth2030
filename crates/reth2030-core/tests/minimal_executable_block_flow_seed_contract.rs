use std::{
    fs,
    path::{Path, PathBuf},
};

use reth2030_types::{
    BlobTx, Block, Eip1559Tx, Header, LegacyTx, LogEntry, Receipt, Transaction, ValidationError,
};

const TODO_ACCEPTANCE_CRITERION_LINE: &str =
    "- [x] Types can represent at least a minimal executable block flow.";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()))
}

fn addr(byte: u8) -> [u8; 20] {
    [byte; 20]
}

fn minimal_header(gas_limit: u64, gas_used: u64) -> Header {
    Header {
        parent_hash: [0; 32],
        number: 1,
        timestamp: 1_762_312_000,
        gas_limit,
        gas_used,
        state_root: [1; 32],
        transactions_root: [2; 32],
        receipts_root: [3; 32],
    }
}

fn minimal_legacy_tx(nonce: u64, from: [u8; 20], to: [u8; 20], value: u128) -> Transaction {
    Transaction::Legacy(LegacyTx {
        nonce,
        from,
        to: Some(to),
        gas_limit: 21_000,
        gas_price: 1,
        value,
        data: Vec::new(),
    })
}

fn minimal_eip1559_tx(nonce: u64, from: [u8; 20], to: [u8; 20], value: u128) -> Transaction {
    Transaction::Eip1559(Eip1559Tx {
        nonce,
        from,
        to: Some(to),
        gas_limit: 21_000,
        max_fee_per_gas: 2,
        max_priority_fee_per_gas: 1,
        value,
        data: vec![0xee],
    })
}

fn minimal_blob_tx(nonce: u64, from: [u8; 20], to: [u8; 20], value: u128) -> Transaction {
    Transaction::Blob(BlobTx {
        nonce,
        from,
        to: Some(to),
        gas_limit: 21_000,
        max_fee_per_gas: 3,
        max_priority_fee_per_gas: 1,
        max_fee_per_blob_gas: 2,
        value,
        data: vec![0xbb],
        blob_versioned_hashes: vec![[0xab; 32]],
    })
}

fn minimal_legacy_contract_creation_tx(nonce: u64, from: [u8; 20], value: u128) -> Transaction {
    Transaction::Legacy(LegacyTx {
        nonce,
        from,
        to: None,
        gas_limit: 21_000,
        gas_price: 1,
        value,
        data: vec![0xca],
    })
}

fn minimal_eip1559_contract_creation_tx(nonce: u64, from: [u8; 20], value: u128) -> Transaction {
    Transaction::Eip1559(Eip1559Tx {
        nonce,
        from,
        to: None,
        gas_limit: 21_000,
        max_fee_per_gas: 2,
        max_priority_fee_per_gas: 1,
        value,
        data: vec![0xcb],
    })
}

fn minimal_blob_contract_creation_tx(nonce: u64, from: [u8; 20], value: u128) -> Transaction {
    Transaction::Blob(BlobTx {
        nonce,
        from,
        to: None,
        gas_limit: 21_000,
        max_fee_per_gas: 3,
        max_priority_fee_per_gas: 1,
        max_fee_per_blob_gas: 2,
        value,
        data: vec![0xcc],
        blob_versioned_hashes: vec![[0xcd; 32]],
    })
}

#[test]
fn todo_marks_minimal_executable_block_flow_acceptance_criterion_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines()
            .any(|line| line.trim() == TODO_ACCEPTANCE_CRITERION_LINE),
        "TODO.md must keep this acceptance criterion checked: {TODO_ACCEPTANCE_CRITERION_LINE}"
    );
}

#[test]
fn types_represent_pre_execution_block_flow_with_empty_receipts() {
    let block = Block {
        header: minimal_header(30_000_000, 0),
        transactions: vec![minimal_legacy_tx(0, addr(0x11), addr(0x22), 7)],
        receipts: Vec::new(),
    };

    assert_eq!(block.validate_basic(), Ok(()));
}

#[test]
fn types_represent_pre_execution_block_flow_with_mixed_tx_variants() {
    let block = Block {
        header: minimal_header(30_000_000, 63_000),
        transactions: vec![
            minimal_legacy_tx(0, addr(0x11), addr(0x22), 7),
            minimal_eip1559_tx(1, addr(0x33), addr(0x44), 9),
            minimal_blob_tx(2, addr(0x55), addr(0x66), 11),
        ],
        receipts: Vec::new(),
    };

    assert_eq!(block.validate_basic(), Ok(()));
}

#[test]
fn types_represent_pre_execution_contract_creation_block_flow_with_mixed_tx_variants() {
    let block = Block {
        header: minimal_header(30_000_000, 63_000),
        transactions: vec![
            minimal_legacy_contract_creation_tx(0, addr(0x21), 7),
            minimal_eip1559_contract_creation_tx(1, addr(0x31), 9),
            minimal_blob_contract_creation_tx(2, addr(0x41), 11),
        ],
        receipts: Vec::new(),
    };

    assert_eq!(block.validate_basic(), Ok(()));
    assert!(block.transactions.iter().all(|tx| tx.to().is_none()));
}

#[test]
fn types_represent_minimal_post_execution_block_flow_and_json_roundtrip() {
    let block = Block {
        header: minimal_header(30_000_000, 21_000),
        transactions: vec![minimal_legacy_tx(0, addr(0x33), addr(0x44), 9)],
        receipts: vec![Receipt {
            tx_hash: [9; 32],
            success: true,
            cumulative_gas_used: 21_000,
            logs: vec![LogEntry {
                address: addr(0x44),
                topics: vec![[7; 32]],
                data: vec![0xaa, 0xbb],
            }],
        }],
    };

    assert_eq!(block.validate_basic(), Ok(()));

    let encoded = block.to_json_bytes().expect("block serialization");
    let decoded = Block::from_json_bytes(&encoded).expect("block deserialization");
    assert_eq!(decoded, block);
    assert_eq!(decoded.validate_basic(), Ok(()));
}

#[test]
fn types_represent_post_execution_contract_creation_block_flow_with_plateau_gas() {
    let block = Block {
        header: minimal_header(30_000_000, 42_000),
        transactions: vec![
            minimal_legacy_contract_creation_tx(0, addr(0x71), 1),
            minimal_eip1559_contract_creation_tx(1, addr(0x72), 2),
            minimal_blob_contract_creation_tx(2, addr(0x73), 3),
        ],
        receipts: vec![
            Receipt {
                tx_hash: [0x71; 32],
                success: true,
                cumulative_gas_used: 21_000,
                logs: Vec::new(),
            },
            Receipt {
                tx_hash: [0x72; 32],
                success: false,
                cumulative_gas_used: 21_000,
                logs: Vec::new(),
            },
            Receipt {
                tx_hash: [0x73; 32],
                success: true,
                cumulative_gas_used: 42_000,
                logs: Vec::new(),
            },
        ],
    };

    assert_eq!(block.validate_basic(), Ok(()));
    assert!(block.transactions.iter().all(|tx| tx.to().is_none()));

    let encoded = block.to_json_bytes().expect("block serialization");
    let decoded = Block::from_json_bytes(&encoded).expect("block deserialization");
    assert_eq!(decoded, block);
    assert_eq!(decoded.validate_basic(), Ok(()));
    assert!(decoded.transactions.iter().all(|tx| tx.to().is_none()));
}

#[test]
fn types_represent_post_execution_block_flow_with_mixed_tx_variants() {
    let block = Block {
        header: minimal_header(30_000_000, 63_000),
        transactions: vec![
            minimal_legacy_tx(0, addr(0x10), addr(0x20), 1),
            minimal_eip1559_tx(1, addr(0x30), addr(0x40), 2),
            minimal_blob_tx(2, addr(0x50), addr(0x60), 3),
        ],
        receipts: vec![
            Receipt {
                tx_hash: [1; 32],
                success: true,
                cumulative_gas_used: 21_000,
                logs: Vec::new(),
            },
            Receipt {
                tx_hash: [2; 32],
                success: true,
                cumulative_gas_used: 42_000,
                logs: Vec::new(),
            },
            Receipt {
                tx_hash: [3; 32],
                success: true,
                cumulative_gas_used: 63_000,
                logs: Vec::new(),
            },
        ],
    };

    assert_eq!(block.validate_basic(), Ok(()));

    let encoded = block.to_json_bytes().expect("block serialization");
    let decoded = Block::from_json_bytes(&encoded).expect("block deserialization");
    assert_eq!(decoded, block);
    assert_eq!(decoded.validate_basic(), Ok(()));
}

#[test]
fn minimal_block_flow_rejects_receipt_count_mismatch() {
    let block = Block {
        header: minimal_header(30_000_000, 21_000),
        transactions: vec![
            minimal_legacy_tx(0, addr(0x01), addr(0x02), 1),
            minimal_legacy_tx(1, addr(0x01), addr(0x03), 2),
        ],
        receipts: vec![Receipt {
            tx_hash: [1; 32],
            success: true,
            cumulative_gas_used: 21_000,
            logs: Vec::new(),
        }],
    };

    assert_eq!(
        block.validate_basic(),
        Err(ValidationError::ReceiptCountMismatch)
    );
}

#[test]
fn minimal_block_flow_rejects_non_monotonic_cumulative_gas() {
    let block = Block {
        header: minimal_header(30_000_000, 20_000),
        transactions: vec![
            minimal_legacy_tx(0, addr(0x01), addr(0x02), 1),
            minimal_legacy_tx(1, addr(0x01), addr(0x03), 2),
        ],
        receipts: vec![
            Receipt {
                tx_hash: [1; 32],
                success: true,
                cumulative_gas_used: 30_000,
                logs: Vec::new(),
            },
            Receipt {
                tx_hash: [2; 32],
                success: true,
                cumulative_gas_used: 20_000,
                logs: Vec::new(),
            },
        ],
    };

    assert_eq!(
        block.validate_basic(),
        Err(ValidationError::ReceiptCumulativeGasNotMonotonic)
    );
}

#[test]
fn minimal_block_flow_rejects_final_receipt_gas_mismatch() {
    let block = Block {
        header: minimal_header(30_000_000, 21_000),
        transactions: vec![minimal_legacy_tx(0, addr(0x01), addr(0x02), 1)],
        receipts: vec![Receipt {
            tx_hash: [1; 32],
            success: true,
            cumulative_gas_used: 20_999,
            logs: Vec::new(),
        }],
    };

    assert_eq!(
        block.validate_basic(),
        Err(ValidationError::ReceiptFinalGasUsedMismatch)
    );
}

#[test]
fn minimal_block_flow_rejects_header_gas_used_above_limit_for_pre_execution_shape() {
    let block = Block {
        header: minimal_header(21_000, 21_001),
        transactions: vec![minimal_legacy_tx(0, addr(0x01), addr(0x02), 1)],
        receipts: Vec::new(),
    };

    assert_eq!(
        block.validate_basic(),
        Err(ValidationError::GasUsedExceedsLimit)
    );
}

#[test]
fn minimal_block_flow_prioritizes_header_gas_limit_violation_over_receipt_checks() {
    let block = Block {
        header: minimal_header(21_000, 21_001),
        transactions: vec![
            minimal_legacy_tx(0, addr(0x01), addr(0x02), 1),
            minimal_eip1559_tx(1, addr(0x03), addr(0x04), 2),
        ],
        receipts: vec![Receipt {
            tx_hash: [1; 32],
            success: true,
            cumulative_gas_used: 21_000,
            logs: Vec::new(),
        }],
    };

    assert_eq!(
        block.validate_basic(),
        Err(ValidationError::GasUsedExceedsLimit)
    );
}

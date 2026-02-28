use std::{
    fs,
    path::{Path, PathBuf},
};

use reth2030_core::{
    Account, ExecutionEngine, ExecutionError, InMemoryState, SimpleExecutionEngine, StateError,
    StateStore,
};
use reth2030_types::{BlobTx, Block, Eip1559Tx, Header, LegacyTx, Transaction, ValidationError};

const TODO_ACCEPTANCE_CRITERION_LINE: &str =
    "- [x] A block execution pipeline exists end-to-end in-process.";
const REQUIRED_EXECUTION_PIPELINE_FRAGMENTS: [&str; 7] = [
    "block.validate_basic()?;",
    "for (index, tx) in block.transactions.iter().enumerate() {",
    "state.apply_transaction(tx)?;",
    "checked_add(gas_used)",
    "if attempted > block.header.gas_limit {",
    "receipts.push(Receipt",
    "Ok(BlockExecutionResult",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()))
}

fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn addr(byte: u8) -> [u8; 20] {
    [byte; 20]
}

fn mk_legacy(from: [u8; 20], to: [u8; 20], nonce: u64, value: u128) -> Transaction {
    Transaction::Legacy(LegacyTx {
        nonce,
        from,
        to: Some(to),
        gas_limit: 30_000,
        gas_price: 1,
        value,
        data: Vec::new(),
    })
}

fn mk_eip1559(from: [u8; 20], to: [u8; 20], nonce: u64, value: u128) -> Transaction {
    Transaction::Eip1559(Eip1559Tx {
        nonce,
        from,
        to: Some(to),
        gas_limit: 30_000,
        max_fee_per_gas: 100,
        max_priority_fee_per_gas: 2,
        value,
        data: vec![0xca, 0xfe],
    })
}

fn mk_blob(from: [u8; 20], to: [u8; 20], nonce: u64, value: u128) -> Transaction {
    Transaction::Blob(BlobTx {
        nonce,
        from,
        to: Some(to),
        gas_limit: 30_000,
        max_fee_per_gas: 120,
        max_priority_fee_per_gas: 3,
        max_fee_per_blob_gas: 10,
        value,
        data: vec![1, 2, 3, 4],
        blob_versioned_hashes: vec![[0x11; 32]],
    })
}

fn block_with_txs_and_gas_limit(txs: Vec<Transaction>, gas_limit: u64) -> Block {
    Block {
        header: Header {
            parent_hash: [0; 32],
            number: 1,
            timestamp: 1_762_312_000,
            gas_limit,
            gas_used: 0,
            state_root: [0; 32],
            transactions_root: [0; 32],
            receipts_root: [0; 32],
        },
        transactions: txs,
        receipts: Vec::new(),
    }
}

#[test]
fn todo_marks_phase2_in_process_pipeline_acceptance_criterion_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines()
            .any(|line| line.trim() == TODO_ACCEPTANCE_CRITERION_LINE),
        "TODO.md must keep this acceptance criterion checked: {TODO_ACCEPTANCE_CRITERION_LINE}"
    );
}

#[test]
fn simple_execution_engine_keeps_end_to_end_pipeline_stages_wired() {
    let source = read_repo_file("crates/reth2030-core/src/execution.rs");
    let normalized = normalize_whitespace(&source);

    for required_fragment in REQUIRED_EXECUTION_PIPELINE_FRAGMENTS {
        assert!(
            normalized.contains(required_fragment),
            "execution pipeline implementation must include `{required_fragment}`"
        );
    }
}

#[test]
fn pipeline_executes_mixed_transactions_end_to_end_in_process() {
    let engine: Box<dyn ExecutionEngine> = Box::new(SimpleExecutionEngine::default());
    let block = block_with_txs_and_gas_limit(
        vec![
            mk_legacy(addr(0x01), addr(0x02), 0, 4),
            mk_eip1559(addr(0x01), addr(0x03), 1, 5),
            mk_blob(addr(0x04), addr(0x05), 0, 6),
        ],
        63_000,
    );

    let mut state = InMemoryState::new();
    state.upsert_account(
        addr(0x01),
        Account {
            balance: 9,
            ..Account::default()
        },
    );
    state.upsert_account(
        addr(0x04),
        Account {
            balance: 6,
            ..Account::default()
        },
    );

    let state_store: &mut dyn StateStore = &mut state;
    let result = engine
        .execute_block(state_store, &block)
        .expect("mixed-variant block should execute");

    assert_eq!(result.total_gas_used, 63_000);
    assert_eq!(result.tx_results.len(), 3);
    assert_eq!(result.receipts.len(), 3);

    assert_eq!(result.tx_results[0].tx_index, 0);
    assert_eq!(result.tx_results[0].gas_used, 21_000);
    assert_eq!(result.tx_results[0].cumulative_gas_used, 21_000);
    assert!(result.tx_results[0].success);

    assert_eq!(result.tx_results[1].tx_index, 1);
    assert_eq!(result.tx_results[1].gas_used, 21_000);
    assert_eq!(result.tx_results[1].cumulative_gas_used, 42_000);
    assert!(result.tx_results[1].success);

    assert_eq!(result.tx_results[2].tx_index, 2);
    assert_eq!(result.tx_results[2].gas_used, 21_000);
    assert_eq!(result.tx_results[2].cumulative_gas_used, 63_000);
    assert!(result.tx_results[2].success);

    assert!(result.receipts.iter().all(|receipt| receipt.success));
    assert_eq!(result.receipts[0].cumulative_gas_used, 21_000);
    assert_eq!(result.receipts[1].cumulative_gas_used, 42_000);
    assert_eq!(result.receipts[2].cumulative_gas_used, 63_000);
    assert_ne!(result.receipts[0].tx_hash, result.receipts[1].tx_hash);
    assert_ne!(result.receipts[1].tx_hash, result.receipts[2].tx_hash);
    assert_ne!(result.receipts[0].tx_hash, result.receipts[2].tx_hash);

    let sender_a = state.get_account(&addr(0x01)).expect("sender A account");
    let sender_b = state.get_account(&addr(0x04)).expect("sender B account");
    let recipient_a = state.get_account(&addr(0x02)).expect("recipient A account");
    let recipient_b = state.get_account(&addr(0x03)).expect("recipient B account");
    let recipient_c = state.get_account(&addr(0x05)).expect("recipient C account");

    assert_eq!(sender_a.balance, 0);
    assert_eq!(sender_a.nonce, 2);
    assert_eq!(sender_b.balance, 0);
    assert_eq!(sender_b.nonce, 1);
    assert_eq!(recipient_a.balance, 4);
    assert_eq!(recipient_b.balance, 5);
    assert_eq!(recipient_c.balance, 6);
}

#[test]
fn pipeline_rejects_invalid_block_before_state_application() {
    let engine = SimpleExecutionEngine::default();
    let mut block =
        block_with_txs_and_gas_limit(vec![mk_legacy(addr(0x01), addr(0x02), 0, 1)], 63_000);
    block.header.gas_used = 63_001;

    let mut state = InMemoryState::new();
    state.upsert_account(
        addr(0x01),
        Account {
            balance: 1,
            ..Account::default()
        },
    );
    let before = state.snapshot();

    let err = engine
        .execute_block(&mut state, &block)
        .expect_err("invalid block must fail before any state mutation");

    assert_eq!(
        err,
        ExecutionError::InvalidBlock(ValidationError::GasUsedExceedsLimit)
    );
    assert_eq!(state.snapshot(), before);
}

#[test]
fn pipeline_stops_at_first_failing_transaction_and_keeps_prior_progress_only() {
    let engine = SimpleExecutionEngine::default();
    let block = block_with_txs_and_gas_limit(
        vec![
            mk_legacy(addr(0x01), addr(0x02), 0, 4),
            mk_legacy(addr(0x01), addr(0x03), 1, 4),
        ],
        63_000,
    );

    let mut state = InMemoryState::new();
    state.upsert_account(
        addr(0x01),
        Account {
            balance: 6,
            ..Account::default()
        },
    );

    let err = engine
        .execute_block(&mut state, &block)
        .expect_err("second tx should fail with insufficient balance");

    assert_eq!(
        err,
        ExecutionError::State(StateError::InsufficientBalance {
            address: addr(0x01),
            available: 2,
            requested: 4,
        })
    );

    let sender = state.get_account(&addr(0x01)).expect("sender account");
    let first_recipient = state
        .get_account(&addr(0x02))
        .expect("first transaction should remain applied");
    assert!(state.get_account(&addr(0x03)).is_none());

    assert_eq!(sender.balance, 2);
    assert_eq!(sender.nonce, 1);
    assert_eq!(first_recipient.balance, 4);
}

#[test]
fn pipeline_rejects_transaction_that_pushes_block_over_gas_limit_without_applying_it() {
    let engine = SimpleExecutionEngine::default();
    let block = block_with_txs_and_gas_limit(
        vec![
            mk_legacy(addr(0x01), addr(0x02), 0, 4),
            mk_legacy(addr(0x01), addr(0x03), 1, 5),
        ],
        21_000,
    );

    let mut state = InMemoryState::new();
    state.upsert_account(
        addr(0x01),
        Account {
            balance: 9,
            ..Account::default()
        },
    );

    let err = engine
        .execute_block(&mut state, &block)
        .expect_err("second tx should fail block-level gas-limit guard");

    assert_eq!(
        err,
        ExecutionError::GasLimitExceeded {
            gas_limit: 21_000,
            attempted: 42_000,
            tx_index: 1,
        }
    );

    let sender = state.get_account(&addr(0x01)).expect("sender account");
    let first_recipient = state
        .get_account(&addr(0x02))
        .expect("first recipient account should be created");
    assert!(
        state.get_account(&addr(0x03)).is_none(),
        "second recipient must not be created when gas limit check fails pre-state"
    );

    assert_eq!(sender.balance, 5);
    assert_eq!(sender.nonce, 1);
    assert_eq!(first_recipient.balance, 4);
}

use std::{
    fs,
    path::{Path, PathBuf},
};

use reth2030_core::{
    Account, ExecutionEngine, ExecutionError, InMemoryState, SimpleExecutionEngine, StateStore,
};
use reth2030_types::{Block, Header, LegacyTx, Transaction};

const TODO_SEED_TASK_LINE: &str =
    "- [x] Implement a scaffold engine (no-op or simplified execution path).";
const REQUIRED_SCAFFOLD_ENGINE_SOURCE_FRAGMENTS: [&str; 8] = [
    "pub struct SimpleExecutionEngine",
    "pub base_gas_per_tx: u64,",
    "impl Default for SimpleExecutionEngine",
    "base_gas_per_tx: 21_000,",
    "pub fn new(base_gas_per_tx: u64) -> Self",
    "fn gas_for_transaction(&self) -> u64",
    "self.base_gas_per_tx",
    "impl ExecutionEngine for SimpleExecutionEngine",
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

fn mk_legacy(from: [u8; 20], to: [u8; 20], nonce: u64, gas_limit: u64, value: u128) -> Transaction {
    Transaction::Legacy(LegacyTx {
        nonce,
        from,
        to: Some(to),
        gas_limit,
        gas_price: 1,
        value,
        data: Vec::new(),
    })
}

fn block_with_txs_and_gas_limit(transactions: Vec<Transaction>, gas_limit: u64) -> Block {
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
        transactions,
        receipts: Vec::new(),
    }
}

#[test]
fn todo_marks_scaffold_engine_seed_task_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines().any(|line| line.trim() == TODO_SEED_TASK_LINE),
        "TODO.md must keep the scaffold-engine seed task checked: {TODO_SEED_TASK_LINE}"
    );
}

#[test]
fn execution_module_keeps_simple_scaffold_engine_wiring() {
    let source = read_repo_file("crates/reth2030-core/src/execution.rs");
    let normalized = normalize_whitespace(&source);

    for fragment in REQUIRED_SCAFFOLD_ENGINE_SOURCE_FRAGMENTS {
        assert!(
            normalized.contains(fragment),
            "execution scaffolding must include `{fragment}`"
        );
    }
}

#[test]
fn scaffold_engine_executes_empty_block_without_side_effects() {
    let engine = SimpleExecutionEngine::default();
    let block = block_with_txs_and_gas_limit(Vec::new(), 0);

    let mut state = InMemoryState::new();
    state.upsert_account(
        addr(0x01),
        Account {
            balance: 11,
            ..Account::default()
        },
    );
    let before = state.snapshot();

    let result = engine
        .execute_block(&mut state, &block)
        .expect("empty block should execute successfully");

    assert_eq!(result.total_gas_used, 0);
    assert!(result.tx_results.is_empty());
    assert!(result.receipts.is_empty());
    assert_eq!(state.snapshot(), before);
}

#[test]
fn scaffold_engine_supports_noop_gas_path_with_zero_base_gas() {
    let engine = SimpleExecutionEngine::new(0);
    let block = block_with_txs_and_gas_limit(vec![mk_legacy(addr(0x01), addr(0x02), 0, 0, 4)], 0);

    let mut state = InMemoryState::new();
    state.upsert_account(
        addr(0x01),
        Account {
            balance: 4,
            ..Account::default()
        },
    );

    let result = engine
        .execute_block(&mut state, &block)
        .expect("zero-base-gas scaffold engine should still apply state transitions");

    assert_eq!(result.total_gas_used, 0);
    assert_eq!(result.tx_results.len(), 1);
    assert_eq!(result.tx_results[0].gas_used, 0);
    assert_eq!(result.tx_results[0].cumulative_gas_used, 0);
    assert_eq!(result.receipts.len(), 1);
    assert_eq!(result.receipts[0].cumulative_gas_used, 0);

    let sender = state.get_account(&addr(0x01)).expect("sender account");
    let recipient = state.get_account(&addr(0x02)).expect("recipient account");
    assert_eq!(sender.balance, 0);
    assert_eq!(sender.nonce, 1);
    assert_eq!(recipient.balance, 4);
}

#[test]
fn scaffold_engine_applies_custom_base_gas_per_transaction() {
    let engine = SimpleExecutionEngine::new(1_000);
    let block = block_with_txs_and_gas_limit(
        vec![
            mk_legacy(addr(0x01), addr(0x02), 0, 1_000, 2),
            mk_legacy(addr(0x01), addr(0x03), 1, 1_000, 3),
            mk_legacy(addr(0x01), addr(0x04), 2, 1_000, 4),
        ],
        3_000,
    );

    let mut state = InMemoryState::new();
    state.upsert_account(
        addr(0x01),
        Account {
            balance: 9,
            ..Account::default()
        },
    );

    let result = engine
        .execute_block(&mut state, &block)
        .expect("all txs should execute at configured scaffold intrinsic gas");

    assert_eq!(result.total_gas_used, 3_000);
    assert_eq!(result.tx_results.len(), 3);
    assert_eq!(result.tx_results[0].gas_used, 1_000);
    assert_eq!(result.tx_results[0].cumulative_gas_used, 1_000);
    assert_eq!(result.tx_results[1].gas_used, 1_000);
    assert_eq!(result.tx_results[1].cumulative_gas_used, 2_000);
    assert_eq!(result.tx_results[2].gas_used, 1_000);
    assert_eq!(result.tx_results[2].cumulative_gas_used, 3_000);
    assert_eq!(result.receipts[0].cumulative_gas_used, 1_000);
    assert_eq!(result.receipts[1].cumulative_gas_used, 2_000);
    assert_eq!(result.receipts[2].cumulative_gas_used, 3_000);

    let sender = state.get_account(&addr(0x01)).expect("sender account");
    assert_eq!(sender.balance, 0);
    assert_eq!(sender.nonce, 3);
}

#[test]
fn scaffold_engine_rejects_tx_when_custom_intrinsic_gas_exceeds_tx_limit() {
    let engine = SimpleExecutionEngine::new(30_000);
    let block = block_with_txs_and_gas_limit(
        vec![mk_legacy(addr(0x01), addr(0x02), 0, 29_999, 1)],
        30_000,
    );

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
        .expect_err("intrinsic gas > tx gas limit should fail before state application");

    assert_eq!(
        err,
        ExecutionError::TxGasLimitTooLow {
            tx_gas_limit: 29_999,
            required: 30_000,
            tx_index: 0,
        }
    );
    assert_eq!(state.snapshot(), before);
}

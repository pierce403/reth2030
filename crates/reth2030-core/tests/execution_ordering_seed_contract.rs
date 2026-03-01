use std::{
    fs,
    path::{Path, PathBuf},
};

use reth2030_core::{
    Account, ExecutionEngine, ExecutionError, InMemoryState, SimpleExecutionEngine, StateError,
    StateStore,
};
use reth2030_types::{Block, Header, LegacyTx, Transaction};

const TODO_TASK_LINE: &str =
    "- [x] Add integration tests for multi-transaction block execution ordering.";
const REQUIRED_EXECUTION_ORDERING_TESTS: [&str; 7] = [
    "fn block_execution_respects_transaction_order()",
    "fn block_execution_order_controls_cross_sender_funding_dependencies()",
    "fn block_execution_halts_at_first_ordered_failure_in_mixed_variant_dependency_chain()",
    "fn block_execution_halts_on_intrinsic_gas_failure_after_partial_progress()",
    "fn block_execution_order_controls_partial_progress_when_intrinsic_failure_is_reordered()",
    "fn block_execution_order_controls_contract_creation_partial_progress()",
    "fn block_execution_order_controls_partial_progress_when_block_gas_limit_is_hit()",
];

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

fn mk_legacy(from: [u8; 20], to: [u8; 20], nonce: u64, value: u128) -> Transaction {
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

fn block_with_txs(txs: Vec<Transaction>) -> Block {
    Block {
        header: Header {
            parent_hash: [0; 32],
            number: 1,
            timestamp: 1_762_312_000,
            gas_limit: 63_000,
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
fn todo_marks_multi_transaction_execution_ordering_task_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines().any(|line| line.trim() == TODO_TASK_LINE),
        "TODO.md must keep this task checked: {TODO_TASK_LINE}"
    );
}

#[test]
fn execution_ordering_suite_keeps_required_multi_transaction_edge_case_tests() {
    let source = read_repo_file("crates/reth2030-core/tests/execution_ordering.rs");
    for required_test in REQUIRED_EXECUTION_ORDERING_TESTS {
        assert!(
            source.contains(required_test),
            "execution_ordering integration suite must include `{required_test}`"
        );
    }
}

#[test]
fn execution_ordering_applies_first_competing_transfer_and_fails_closed_afterward() {
    let engine = SimpleExecutionEngine::default();
    let tx_to_first = mk_legacy(addr(0x51), addr(0x61), 0, 7);
    let tx_to_second = mk_legacy(addr(0x51), addr(0x62), 1, 5);

    let block_first_then_second = block_with_txs(vec![tx_to_first.clone(), tx_to_second.clone()]);
    let block_second_then_first = block_with_txs(vec![tx_to_second, tx_to_first]);

    let mut state_a = InMemoryState::new();
    state_a.upsert_account(
        addr(0x51),
        Account {
            balance: 7,
            ..Account::default()
        },
    );
    let mut state_b = state_a.clone();

    let err_a = engine
        .execute_block(&mut state_a, &block_first_then_second)
        .expect_err("second transfer should fail after first drains sender");
    let err_b = engine
        .execute_block(&mut state_b, &block_second_then_first)
        .expect_err("reordered second transfer should fail after first drains sender");

    assert_eq!(
        err_a,
        ExecutionError::State(StateError::InsufficientBalance {
            address: addr(0x51),
            available: 0,
            requested: 5,
        })
    );
    assert_eq!(
        err_b,
        ExecutionError::State(StateError::InsufficientBalance {
            address: addr(0x51),
            available: 2,
            requested: 7,
        })
    );

    let sender_a = state_a
        .get_account(&addr(0x51))
        .expect("sender must remain in state");
    let sender_b = state_b
        .get_account(&addr(0x51))
        .expect("sender must remain in state");
    assert_eq!(sender_a.balance, 0);
    assert_eq!(sender_a.nonce, 1);
    assert_eq!(sender_b.balance, 2);
    assert_eq!(sender_b.nonce, 1);

    let recipient_first = state_a
        .get_account(&addr(0x61))
        .expect("first recipient in order A should receive funds");
    let recipient_second = state_b
        .get_account(&addr(0x62))
        .expect("first recipient in order B should receive funds");
    assert_eq!(recipient_first.balance, 7);
    assert_eq!(recipient_second.balance, 5);
    assert!(
        state_a.get_account(&addr(0x62)).is_none(),
        "recipient of failing tx in order A must not be created"
    );
    assert!(
        state_b.get_account(&addr(0x61)).is_none(),
        "recipient of failing tx in order B must not be created"
    );
}

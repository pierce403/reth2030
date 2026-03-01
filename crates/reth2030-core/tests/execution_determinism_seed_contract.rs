use std::{
    fs,
    path::{Path, PathBuf},
};

use reth2030_core::{
    Account, ExecutionEngine, ExecutionError, InMemoryState, SimpleExecutionEngine, StateError,
    StateStore,
};
use reth2030_types::{BlobTx, Block, Eip1559Tx, Header, LegacyTx, Transaction};

const TODO_ACCEPTANCE_CRITERION_LINE: &str =
    "- [x] Execution output is deterministic under repeated runs.";

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

fn mk_legacy_contract_creation(
    from: [u8; 20],
    nonce: u64,
    gas_limit: u64,
    value: u128,
    data: Vec<u8>,
) -> Transaction {
    Transaction::Legacy(LegacyTx {
        nonce,
        from,
        to: None,
        gas_limit,
        gas_price: 1,
        value,
        data,
    })
}

fn mk_eip1559_contract_creation(
    from: [u8; 20],
    nonce: u64,
    gas_limit: u64,
    value: u128,
    data: Vec<u8>,
) -> Transaction {
    Transaction::Eip1559(Eip1559Tx {
        nonce,
        from,
        to: None,
        gas_limit,
        max_fee_per_gas: 100,
        max_priority_fee_per_gas: 2,
        value,
        data,
    })
}

fn mk_blob_contract_creation(
    from: [u8; 20],
    nonce: u64,
    gas_limit: u64,
    value: u128,
    data: Vec<u8>,
    blob_versioned_hashes: Vec<[u8; 32]>,
) -> Transaction {
    Transaction::Blob(BlobTx {
        nonce,
        from,
        to: None,
        gas_limit,
        max_fee_per_gas: 120,
        max_priority_fee_per_gas: 3,
        max_fee_per_blob_gas: 10,
        value,
        data,
        blob_versioned_hashes,
    })
}

fn mk_legacy_transfer(
    from: [u8; 20],
    to: [u8; 20],
    nonce: u64,
    gas_limit: u64,
    value: u128,
) -> Transaction {
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

fn mk_eip1559_transfer(
    from: [u8; 20],
    to: [u8; 20],
    nonce: u64,
    gas_limit: u64,
    value: u128,
    data: Vec<u8>,
) -> Transaction {
    Transaction::Eip1559(Eip1559Tx {
        nonce,
        from,
        to: Some(to),
        gas_limit,
        max_fee_per_gas: 100,
        max_priority_fee_per_gas: 2,
        value,
        data,
    })
}

fn mk_blob_transfer(
    from: [u8; 20],
    to: [u8; 20],
    nonce: u64,
    gas_limit: u64,
    value: u128,
    data: Vec<u8>,
    blob_versioned_hashes: Vec<[u8; 32]>,
) -> Transaction {
    Transaction::Blob(BlobTx {
        nonce,
        from,
        to: Some(to),
        gas_limit,
        max_fee_per_gas: 120,
        max_priority_fee_per_gas: 3,
        max_fee_per_blob_gas: 10,
        value,
        data,
        blob_versioned_hashes,
    })
}

#[test]
fn todo_marks_execution_determinism_acceptance_criterion_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines()
            .any(|line| line.trim() == TODO_ACCEPTANCE_CRITERION_LINE),
        "TODO.md must keep this acceptance criterion checked: {TODO_ACCEPTANCE_CRITERION_LINE}"
    );
}

#[test]
fn repeated_execution_output_is_deterministic_for_contract_creation_mixed_variants() {
    let block = block_with_txs_and_gas_limit(
        vec![
            mk_legacy_contract_creation(addr(0x01), 0, 30_000, 7, vec![0xde, 0xad]),
            mk_eip1559_contract_creation(addr(0x01), 1, 40_000, 5, vec![0xca, 0xfe]),
            mk_blob_contract_creation(
                addr(0x04),
                0,
                50_000,
                9,
                vec![1, 2, 3, 4],
                vec![[0x11; 32], [0x22; 32]],
            ),
        ],
        75_000,
    );

    let mut initial_state = InMemoryState::new();
    initial_state.upsert_account(
        addr(0x01),
        Account {
            balance: 20,
            ..Account::default()
        },
    );
    initial_state.upsert_account(
        addr(0x04),
        Account {
            balance: 15,
            ..Account::default()
        },
    );

    let engine = SimpleExecutionEngine::default();
    let mut baseline_state = initial_state.clone();
    let baseline_result = engine
        .execute_block(&mut baseline_state, &block)
        .expect("baseline contract-creation execution");
    let baseline_snapshot = baseline_state.snapshot();

    for run in 0..32 {
        let mut run_state = initial_state.clone();
        let run_result = engine
            .execute_block(&mut run_state, &block)
            .expect("repeated contract-creation execution");
        assert_eq!(
            run_result, baseline_result,
            "run {run} produced non-deterministic execution output"
        );
        assert_eq!(
            run_state.snapshot(),
            baseline_snapshot,
            "run {run} produced non-deterministic post-state snapshot"
        );
    }

    assert_eq!(baseline_result.total_gas_used, 63_000);
    assert_eq!(baseline_result.tx_results.len(), 3);
    assert_eq!(baseline_result.receipts.len(), 3);
    assert_ne!(
        baseline_result.receipts[0].tx_hash,
        baseline_result.receipts[1].tx_hash
    );
    assert_ne!(
        baseline_result.receipts[1].tx_hash,
        baseline_result.receipts[2].tx_hash
    );
    assert_ne!(
        baseline_result.receipts[0].tx_hash,
        baseline_result.receipts[2].tx_hash
    );
    assert!(
        baseline_snapshot.contains_key(&addr(0x01)),
        "sender account should persist in post-state snapshot"
    );
    assert!(
        baseline_snapshot.contains_key(&addr(0x04)),
        "sender account should persist in post-state snapshot"
    );
    assert_eq!(
        baseline_snapshot.len(),
        2,
        "contract-creation transactions should not create recipient accounts"
    );
}

#[test]
fn repeated_execution_failure_is_deterministic_for_block_gas_limit_exceeded() {
    let engine = SimpleExecutionEngine::new(30_000);
    let block = block_with_txs_and_gas_limit(
        vec![
            mk_legacy_transfer(addr(0x10), addr(0x20), 0, 30_000, 4),
            mk_legacy_transfer(addr(0x10), addr(0x30), 1, 30_000, 3),
        ],
        50_000,
    );

    let mut initial_state = InMemoryState::new();
    initial_state.upsert_account(
        addr(0x10),
        Account {
            balance: 10,
            ..Account::default()
        },
    );

    let mut baseline_state = initial_state.clone();
    let baseline_err = engine
        .execute_block(&mut baseline_state, &block)
        .expect_err("baseline execution should fail at block gas limit");
    assert_eq!(
        baseline_err,
        ExecutionError::GasLimitExceeded {
            gas_limit: 50_000,
            attempted: 60_000,
            tx_index: 1,
        }
    );
    let baseline_snapshot = baseline_state.snapshot();

    for run in 0..32 {
        let mut run_state = initial_state.clone();
        let run_err = engine
            .execute_block(&mut run_state, &block)
            .expect_err("repeated execution should fail with identical gas-limit error");
        assert_eq!(
            run_err, baseline_err,
            "run {run} produced non-deterministic execution error"
        );
        assert_eq!(
            run_state.snapshot(),
            baseline_snapshot,
            "run {run} produced non-deterministic partial post-state"
        );
    }

    let sender = baseline_snapshot
        .get(&addr(0x10))
        .expect("sender should remain in partial post-state");
    assert_eq!(sender.balance, 6);
    assert_eq!(sender.nonce, 1);
    let first_recipient = baseline_snapshot
        .get(&addr(0x20))
        .expect("first tx recipient should be present");
    assert_eq!(first_recipient.balance, 4);
    assert!(
        !baseline_snapshot.contains_key(&addr(0x30)),
        "failing tx recipient should not be created"
    );
}

#[test]
fn repeated_execution_pre_state_failures_are_deterministic_and_fail_closed() {
    let engine = SimpleExecutionEngine::default();
    let block = block_with_txs_and_gas_limit(
        vec![mk_legacy_transfer(addr(0x40), addr(0x50), 0, 20_999, 1)],
        63_000,
    );

    let mut initial_state = InMemoryState::new();
    initial_state.upsert_account(
        addr(0x40),
        Account {
            balance: 5,
            ..Account::default()
        },
    );
    let baseline_snapshot = initial_state.snapshot();

    for run in 0..32 {
        let mut run_state = initial_state.clone();
        let run_err = engine
            .execute_block(&mut run_state, &block)
            .expect_err("intrinsic-gas precheck should fail deterministically");
        assert_eq!(
            run_err,
            ExecutionError::TxGasLimitTooLow {
                tx_gas_limit: 20_999,
                required: 21_000,
                tx_index: 0,
            },
            "run {run} produced non-deterministic intrinsic-gas error"
        );
        assert_eq!(
            run_state.snapshot(),
            baseline_snapshot,
            "run {run} should preserve fail-closed pre-state snapshot"
        );
    }
}

#[test]
fn repeated_execution_state_failures_are_deterministic_through_dyn_dispatch() {
    let engine: Box<dyn ExecutionEngine> = Box::new(SimpleExecutionEngine::default());
    let block = block_with_txs_and_gas_limit(
        vec![
            mk_legacy_transfer(addr(0x81), addr(0x82), 0, 21_000, 6),
            mk_eip1559_transfer(addr(0x82), addr(0x83), 0, 21_000, 4, vec![0xaa]),
            mk_blob_transfer(
                addr(0x83),
                addr(0x84),
                0,
                21_000,
                5,
                vec![0xbb, 0xcc],
                vec![[0x34; 32]],
            ),
        ],
        84_000,
    );

    let mut initial_state = InMemoryState::new();
    initial_state.upsert_account(
        addr(0x81),
        Account {
            balance: 6,
            ..Account::default()
        },
    );

    let mut baseline_state = initial_state.clone();
    let baseline_store: &mut dyn StateStore = &mut baseline_state;
    let baseline_err = engine
        .execute_block(baseline_store, &block)
        .expect_err("baseline execution should fail on mixed-variant state transition");
    assert_eq!(
        baseline_err,
        ExecutionError::State(StateError::InsufficientBalance {
            address: addr(0x83),
            available: 4,
            requested: 5,
        })
    );
    let baseline_snapshot = baseline_state.snapshot();

    for run in 0..32 {
        let mut run_state = initial_state.clone();
        let run_store: &mut dyn StateStore = &mut run_state;
        let run_err = engine
            .execute_block(run_store, &block)
            .expect_err("repeated execution should fail with identical state-transition error");
        assert_eq!(
            run_err, baseline_err,
            "run {run} produced non-deterministic state-transition error"
        );
        assert_eq!(
            run_state.snapshot(),
            baseline_snapshot,
            "run {run} produced non-deterministic partial post-state"
        );
    }

    let sender_a = baseline_snapshot
        .get(&addr(0x81))
        .expect("first sender should persist in partial post-state");
    let sender_b = baseline_snapshot
        .get(&addr(0x82))
        .expect("intermediate sender should persist in partial post-state");
    let sender_c = baseline_snapshot
        .get(&addr(0x83))
        .expect("failing sender should persist without mutation from failing tx");
    assert_eq!(sender_a.balance, 0);
    assert_eq!(sender_a.nonce, 1);
    assert_eq!(sender_b.balance, 2);
    assert_eq!(sender_b.nonce, 1);
    assert_eq!(sender_c.balance, 4);
    assert_eq!(sender_c.nonce, 0);
    assert!(
        !baseline_snapshot.contains_key(&addr(0x84)),
        "recipient of failing transaction should not be created"
    );
}

#[test]
fn repeated_execution_failure_is_deterministic_for_cumulative_gas_overflow() {
    let engine = SimpleExecutionEngine::new(u64::MAX);
    let block = block_with_txs_and_gas_limit(
        vec![
            mk_legacy_transfer(addr(0x70), addr(0x71), 0, u64::MAX, 1),
            mk_legacy_transfer(addr(0x70), addr(0x72), 1, u64::MAX, 1),
        ],
        u64::MAX,
    );

    let mut initial_state = InMemoryState::new();
    initial_state.upsert_account(
        addr(0x70),
        Account {
            balance: 2,
            ..Account::default()
        },
    );

    let mut baseline_state = initial_state.clone();
    let baseline_err = engine
        .execute_block(&mut baseline_state, &block)
        .expect_err("baseline execution should fail on cumulative gas overflow");
    assert_eq!(
        baseline_err,
        ExecutionError::GasOverflow {
            cumulative_gas: u64::MAX,
            gas_used: u64::MAX,
            tx_index: 1,
        }
    );
    let baseline_snapshot = baseline_state.snapshot();

    for run in 0..32 {
        let mut run_state = initial_state.clone();
        let run_err = engine
            .execute_block(&mut run_state, &block)
            .expect_err("repeated execution should fail with identical gas-overflow error");
        assert_eq!(
            run_err, baseline_err,
            "run {run} produced non-deterministic gas-overflow error"
        );
        assert_eq!(
            run_state.snapshot(),
            baseline_snapshot,
            "run {run} produced non-deterministic partial post-state"
        );
    }

    let sender = baseline_snapshot
        .get(&addr(0x70))
        .expect("sender should remain in partial post-state");
    assert_eq!(sender.balance, 1);
    assert_eq!(sender.nonce, 1);
    let first_recipient = baseline_snapshot
        .get(&addr(0x71))
        .expect("first recipient should be present");
    assert_eq!(first_recipient.balance, 1);
    assert!(
        !baseline_snapshot.contains_key(&addr(0x72)),
        "overflowing tx recipient should not be created"
    );
}

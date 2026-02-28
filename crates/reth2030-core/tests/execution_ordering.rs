use reth2030_core::{
    Account, ExecutionEngine, ExecutionError, InMemoryState, SimpleExecutionEngine, StateError,
    StateStore,
};
use reth2030_types::{BlobTx, Block, Eip1559Tx, Header, LegacyTx, Transaction};

fn addr(byte: u8) -> [u8; 20] {
    [byte; 20]
}

fn mk_legacy(from: [u8; 20], to: [u8; 20], nonce: u64, value: u128) -> Transaction {
    mk_legacy_with_payload(from, to, nonce, 21_000, value, Vec::new())
}

fn mk_legacy_with_gas(
    from: [u8; 20],
    to: [u8; 20],
    nonce: u64,
    gas_limit: u64,
    value: u128,
) -> Transaction {
    mk_legacy_with_payload(from, to, nonce, gas_limit, value, Vec::new())
}

fn mk_legacy_with_payload(
    from: [u8; 20],
    to: [u8; 20],
    nonce: u64,
    gas_limit: u64,
    value: u128,
    data: Vec<u8>,
) -> Transaction {
    Transaction::Legacy(LegacyTx {
        nonce,
        from,
        to: Some(to),
        gas_limit,
        gas_price: 1,
        value,
        data,
    })
}

fn mk_eip1559(
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

fn mk_blob(
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

fn block_with_txs(txs: Vec<Transaction>) -> Block {
    block_with_txs_and_gas_limit(txs, 60_000)
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
        receipts: vec![],
        transactions: txs,
    }
}

#[test]
fn block_execution_is_deterministic_for_identical_input() {
    let txs = vec![
        mk_legacy(addr(0x01), addr(0x02), 0, 10),
        mk_legacy(addr(0x01), addr(0x03), 1, 5),
    ];
    let block = block_with_txs(txs);

    let mut state_a = InMemoryState::new();
    state_a.upsert_account(
        addr(0x01),
        Account {
            balance: 30,
            ..Account::default()
        },
    );
    let mut state_b = state_a.clone();

    let engine = SimpleExecutionEngine::default();

    let result_a = engine
        .execute_block(&mut state_a, &block)
        .expect("first execution");
    let result_b = engine
        .execute_block(&mut state_b, &block)
        .expect("second execution");

    assert_eq!(result_a, result_b);
    assert_eq!(state_a.snapshot(), state_b.snapshot());
}

#[test]
fn repeated_execution_output_is_deterministic_for_mixed_transaction_variants() {
    let block = block_with_txs_and_gas_limit(
        vec![
            mk_legacy_with_payload(
                addr(0x01),
                addr(0x02),
                0,
                30_000,
                7,
                vec![0xde, 0xad, 0xbe, 0xef],
            ),
            mk_eip1559(addr(0x01), addr(0x03), 1, 40_000, 5, vec![0xca, 0xfe]),
            mk_blob(
                addr(0x04),
                addr(0x05),
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
        .expect("baseline mixed-variant execution");
    let baseline_snapshot = baseline_state.snapshot();

    for run in 0..16 {
        let mut run_state = initial_state.clone();
        let run_result = engine
            .execute_block(&mut run_state, &block)
            .expect("repeated mixed-variant execution");
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
}

#[test]
fn repeated_execution_failure_is_deterministic_for_identical_input() {
    let block = block_with_txs(vec![
        mk_legacy(addr(0x01), addr(0x02), 0, 4),
        mk_legacy(addr(0x01), addr(0x03), 1, 4),
    ]);

    let mut initial_state = InMemoryState::new();
    initial_state.upsert_account(
        addr(0x01),
        Account {
            balance: 6,
            ..Account::default()
        },
    );

    let engine = SimpleExecutionEngine::default();
    let mut baseline_state = initial_state.clone();
    let baseline_err = engine
        .execute_block(&mut baseline_state, &block)
        .expect_err("baseline execution should fail on second tx");
    assert_eq!(
        baseline_err,
        ExecutionError::State(StateError::InsufficientBalance {
            address: addr(0x01),
            available: 2,
            requested: 4,
        })
    );
    let baseline_snapshot = baseline_state.snapshot();

    for run in 0..16 {
        let mut run_state = initial_state.clone();
        let run_err = engine
            .execute_block(&mut run_state, &block)
            .expect_err("repeated execution should fail the same way");
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
}

#[test]
fn block_execution_allows_total_gas_equal_to_block_gas_limit() {
    let engine = SimpleExecutionEngine::default();
    let txs = vec![
        mk_legacy(addr(0x01), addr(0x02), 0, 2),
        mk_legacy(addr(0x01), addr(0x03), 1, 3),
    ];
    let block = block_with_txs_and_gas_limit(txs, 42_000);

    let mut state = InMemoryState::new();
    state.upsert_account(
        addr(0x01),
        Account {
            balance: 10,
            ..Account::default()
        },
    );

    let result = engine
        .execute_block(&mut state, &block)
        .expect("block should fit exactly in gas limit");

    assert_eq!(result.total_gas_used, 42_000);
    assert_eq!(result.tx_results.len(), 2);
}

#[test]
fn block_execution_fails_when_tx_gas_limit_is_below_intrinsic_cost() {
    let engine = SimpleExecutionEngine::default();
    let block = block_with_txs(vec![mk_legacy_with_gas(
        addr(0x01),
        addr(0x02),
        0,
        20_999,
        1,
    )]);

    let mut state = InMemoryState::new();
    state.upsert_account(
        addr(0x01),
        Account {
            balance: 5,
            ..Account::default()
        },
    );

    let err = engine
        .execute_block(&mut state, &block)
        .expect_err("tx gas limit must be validated before applying state changes");

    assert_eq!(
        err,
        ExecutionError::TxGasLimitTooLow {
            tx_gas_limit: 20_999,
            required: 21_000,
            tx_index: 0,
        }
    );
    let sender = state
        .get_account(&addr(0x01))
        .expect("sender account should remain untouched");
    assert_eq!(sender.balance, 5);
    assert_eq!(sender.nonce, 0);
    assert!(
        state.get_account(&addr(0x02)).is_none(),
        "recipient account should not be created on pre-state failure"
    );
}

#[test]
fn block_execution_fails_on_cumulative_gas_overflow() {
    let engine = SimpleExecutionEngine::new(u64::MAX);
    let txs = vec![
        mk_legacy_with_gas(addr(0x01), addr(0x02), 0, u64::MAX, 1),
        mk_legacy_with_gas(addr(0x01), addr(0x03), 1, u64::MAX, 1),
    ];
    let block = block_with_txs_and_gas_limit(txs, u64::MAX);

    let mut state = InMemoryState::new();
    state.upsert_account(
        addr(0x01),
        Account {
            balance: 3,
            ..Account::default()
        },
    );

    let err = engine
        .execute_block(&mut state, &block)
        .expect_err("second tx should overflow cumulative gas accounting");
    assert_eq!(
        err,
        ExecutionError::GasOverflow {
            cumulative_gas: u64::MAX,
            gas_used: u64::MAX,
            tx_index: 1,
        }
    );

    let sender = state.get_account(&addr(0x01)).expect("sender account");
    assert_eq!(sender.balance, 2);
    assert_eq!(sender.nonce, 1);
    let first_recipient = state
        .get_account(&addr(0x02))
        .expect("first tx should apply before overflow on second tx");
    assert_eq!(first_recipient.balance, 1);
    assert!(
        state.get_account(&addr(0x03)).is_none(),
        "overflowing tx should not mutate recipient state"
    );
}

#[test]
fn block_execution_respects_transaction_order() {
    let engine = SimpleExecutionEngine::default();

    let tx1 = mk_legacy(addr(0x01), addr(0x02), 0, 20);
    let tx2 = mk_legacy(addr(0x01), addr(0x03), 1, 15);

    let block_order_a = block_with_txs(vec![tx1.clone(), tx2.clone()]);
    let block_order_b = block_with_txs(vec![tx2, tx1]);

    let mut state_a = InMemoryState::new();
    state_a.upsert_account(
        addr(0x01),
        Account {
            balance: 30,
            ..Account::default()
        },
    );

    let mut state_b = state_a.clone();

    let err_a = engine
        .execute_block(&mut state_a, &block_order_a)
        .expect_err("expected second tx to fail in order A");
    let err_b = engine
        .execute_block(&mut state_b, &block_order_b)
        .expect_err("expected second tx to fail in order B");

    assert!(matches!(
        err_a,
        ExecutionError::State(StateError::InsufficientBalance { .. })
    ));
    assert!(matches!(
        err_b,
        ExecutionError::State(StateError::InsufficientBalance { .. })
    ));

    let recipient_a = state_a
        .get_account(&addr(0x02))
        .expect("recipient in order A should get first transfer");
    assert_eq!(recipient_a.balance, 20);

    let recipient_b = state_b
        .get_account(&addr(0x03))
        .expect("recipient in order B should get first transfer");
    assert_eq!(recipient_b.balance, 15);
}

#[test]
fn receipt_hash_is_stable_for_same_tx_across_positions() {
    let engine = SimpleExecutionEngine::default();
    let filler = mk_legacy(addr(0x10), addr(0x11), 0, 1);
    let target = mk_legacy(addr(0x20), addr(0x21), 0, 1);

    let block_a = block_with_txs(vec![filler.clone(), target.clone()]);
    let block_b = block_with_txs(vec![target, filler]);

    let mut state_a = InMemoryState::new();
    state_a.upsert_account(
        addr(0x10),
        Account {
            balance: 5,
            ..Account::default()
        },
    );
    state_a.upsert_account(
        addr(0x20),
        Account {
            balance: 5,
            ..Account::default()
        },
    );
    let mut state_b = state_a.clone();

    let result_a = engine
        .execute_block(&mut state_a, &block_a)
        .expect("block A execution");
    let result_b = engine
        .execute_block(&mut state_b, &block_b)
        .expect("block B execution");

    assert_eq!(result_a.receipts[1].tx_hash, result_b.receipts[0].tx_hash);
}

#[test]
fn receipt_hash_changes_when_tx_content_changes_with_same_sender_nonce() {
    let engine = SimpleExecutionEngine::default();
    let tx_a = mk_legacy(addr(0x33), addr(0x44), 0, 4);
    let tx_b = mk_legacy(addr(0x33), addr(0x55), 0, 5);

    let block_a = block_with_txs(vec![tx_a]);
    let block_b = block_with_txs(vec![tx_b]);

    let mut state_a = InMemoryState::new();
    state_a.upsert_account(
        addr(0x33),
        Account {
            balance: 10,
            ..Account::default()
        },
    );
    let mut state_b = state_a.clone();

    let result_a = engine
        .execute_block(&mut state_a, &block_a)
        .expect("block A execution");
    let result_b = engine
        .execute_block(&mut state_b, &block_b)
        .expect("block B execution");

    assert_ne!(result_a.receipts[0].tx_hash, result_b.receipts[0].tx_hash);
}

#[test]
fn execution_engine_trait_supports_dyn_dispatch_with_state_trait_object() {
    let engine: Box<dyn ExecutionEngine> = Box::new(SimpleExecutionEngine::default());
    let block = block_with_txs_and_gas_limit(vec![mk_legacy(addr(0x01), addr(0x02), 0, 7)], 21_000);

    let mut state = InMemoryState::new();
    state.upsert_account(
        addr(0x01),
        Account {
            balance: 7,
            ..Account::default()
        },
    );

    let state_store: &mut dyn StateStore = &mut state;
    let result = engine
        .execute_block(state_store, &block)
        .expect("dyn-dispatched engine execution");

    assert_eq!(result.total_gas_used, 21_000);
    assert_eq!(result.tx_results.len(), 1);
    assert_eq!(result.receipts.len(), 1);

    let sender = state.get_account(&addr(0x01)).expect("sender account");
    let recipient = state.get_account(&addr(0x02)).expect("recipient account");
    assert_eq!(sender.balance, 0);
    assert_eq!(sender.nonce, 1);
    assert_eq!(recipient.balance, 7);
}

#[test]
fn dyn_dispatched_engine_keeps_pre_state_failures_fail_closed() {
    let engine: Box<dyn ExecutionEngine> = Box::new(SimpleExecutionEngine::default());
    let block = block_with_txs(vec![mk_legacy_with_gas(
        addr(0x01),
        addr(0x02),
        0,
        20_999,
        1,
    )]);

    let mut state = InMemoryState::new();
    state.upsert_account(
        addr(0x01),
        Account {
            balance: 5,
            ..Account::default()
        },
    );

    let state_store: &mut dyn StateStore = &mut state;
    let err = engine
        .execute_block(state_store, &block)
        .expect_err("pre-state gas validation should fail through trait-object dispatch");

    assert_eq!(
        err,
        ExecutionError::TxGasLimitTooLow {
            tx_gas_limit: 20_999,
            required: 21_000,
            tx_index: 0,
        }
    );

    let sender = state
        .get_account(&addr(0x01))
        .expect("sender account should remain unchanged");
    assert_eq!(sender.balance, 5);
    assert_eq!(sender.nonce, 0);
    assert!(
        state.get_account(&addr(0x02)).is_none(),
        "recipient account should not be created when intrinsic gas check fails"
    );
}

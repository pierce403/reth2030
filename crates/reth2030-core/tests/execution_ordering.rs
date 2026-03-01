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
fn block_execution_order_controls_cross_sender_funding_dependencies() {
    let engine = SimpleExecutionEngine::default();

    let fund = mk_legacy(addr(0xa1), addr(0xb1), 0, 8);
    let spend = mk_legacy(addr(0xb1), addr(0xc1), 0, 8);

    let ordered_block = block_with_txs(vec![fund.clone(), spend.clone()]);
    let reversed_block = block_with_txs(vec![spend, fund]);

    let mut ordered_state = InMemoryState::new();
    ordered_state.upsert_account(
        addr(0xa1),
        Account {
            balance: 8,
            ..Account::default()
        },
    );
    let mut reversed_state = ordered_state.clone();

    let ordered_result = engine
        .execute_block(&mut ordered_state, &ordered_block)
        .expect("ordered block should execute when funding precedes dependent spend");
    assert_eq!(ordered_result.tx_results.len(), 2);
    assert_eq!(ordered_result.receipts.len(), 2);
    assert_eq!(ordered_result.tx_results[0].tx_index, 0);
    assert_eq!(ordered_result.tx_results[1].tx_index, 1);

    let sender_a = ordered_state
        .get_account(&addr(0xa1))
        .expect("funder should be debited");
    let sender_b = ordered_state
        .get_account(&addr(0xb1))
        .expect("dependent sender should be created and debited");
    let recipient_c = ordered_state
        .get_account(&addr(0xc1))
        .expect("final recipient should receive funds");
    assert_eq!(sender_a.balance, 0);
    assert_eq!(sender_a.nonce, 1);
    assert_eq!(sender_b.balance, 0);
    assert_eq!(sender_b.nonce, 1);
    assert_eq!(recipient_c.balance, 8);

    let reversed_err = engine
        .execute_block(&mut reversed_state, &reversed_block)
        .expect_err("reordered block should fail before funding-dependent spend");
    assert_eq!(
        reversed_err,
        ExecutionError::State(StateError::InsufficientBalance {
            address: addr(0xb1),
            available: 0,
            requested: 8,
        })
    );

    let reversed_funder = reversed_state
        .get_account(&addr(0xa1))
        .expect("funder account should remain unchanged on early failure");
    assert_eq!(reversed_funder.balance, 8);
    assert_eq!(reversed_funder.nonce, 0);
    assert!(
        reversed_state.get_account(&addr(0xb1)).is_none(),
        "dependent sender account must not be created when reordered spend fails first"
    );
    assert!(
        reversed_state.get_account(&addr(0xc1)).is_none(),
        "recipient account must not be created when reordered spend fails first"
    );
}

#[test]
fn block_execution_halts_at_first_ordered_failure_in_mixed_variant_dependency_chain() {
    let engine = SimpleExecutionEngine::default();
    let block = block_with_txs_and_gas_limit(
        vec![
            mk_legacy(addr(0xd1), addr(0xd2), 0, 9),
            mk_eip1559(addr(0xd2), addr(0xd3), 0, 21_000, 4, vec![0xaa]),
            mk_blob(
                addr(0xd3),
                addr(0xd4),
                0,
                21_000,
                5,
                vec![0xbb, 0xcc],
                vec![[0x12; 32]],
            ),
            mk_legacy(addr(0xd2), addr(0xd5), 1, 1),
        ],
        84_000,
    );

    let mut state = InMemoryState::new();
    state.upsert_account(
        addr(0xd1),
        Account {
            balance: 9,
            ..Account::default()
        },
    );

    let err = engine
        .execute_block(&mut state, &block)
        .expect_err("third transaction should fail and halt later execution");
    assert_eq!(
        err,
        ExecutionError::State(StateError::InsufficientBalance {
            address: addr(0xd3),
            available: 4,
            requested: 5,
        })
    );

    let account_a = state
        .get_account(&addr(0xd1))
        .expect("original sender should reflect first transfer");
    let account_b = state
        .get_account(&addr(0xd2))
        .expect("intermediate account should retain partial progress");
    let account_c = state
        .get_account(&addr(0xd3))
        .expect("second transfer recipient should persist despite later failure");
    assert_eq!(account_a.balance, 0);
    assert_eq!(account_a.nonce, 1);
    assert_eq!(account_b.balance, 5);
    assert_eq!(account_b.nonce, 1);
    assert_eq!(account_c.balance, 4);
    assert_eq!(account_c.nonce, 0);
    assert!(
        state.get_account(&addr(0xd4)).is_none(),
        "failing transaction recipient must remain absent"
    );
    assert!(
        state.get_account(&addr(0xd5)).is_none(),
        "transactions after first failure must not execute"
    );
}

#[test]
fn block_execution_halts_on_intrinsic_gas_failure_after_partial_progress() {
    let engine = SimpleExecutionEngine::default();
    let block = block_with_txs_and_gas_limit(
        vec![
            mk_legacy(addr(0xe1), addr(0xe2), 0, 6),
            mk_eip1559(addr(0xe2), addr(0xe3), 0, 20_999, 1, vec![0xaa]),
            mk_blob(
                addr(0xe2),
                addr(0xe4),
                1,
                21_000,
                2,
                vec![0xbb, 0xcc],
                vec![[0x34; 32]],
            ),
        ],
        84_000,
    );

    let mut state = InMemoryState::new();
    state.upsert_account(
        addr(0xe1),
        Account {
            balance: 6,
            ..Account::default()
        },
    );

    let err = engine
        .execute_block(&mut state, &block)
        .expect_err("second transaction should fail intrinsic gas validation");
    assert_eq!(
        err,
        ExecutionError::TxGasLimitTooLow {
            tx_gas_limit: 20_999,
            required: 21_000,
            tx_index: 1,
        }
    );

    let account_a = state
        .get_account(&addr(0xe1))
        .expect("first sender should reflect successful first transfer");
    let account_b = state
        .get_account(&addr(0xe2))
        .expect("first recipient should persist after later failure");
    assert_eq!(account_a.balance, 0);
    assert_eq!(account_a.nonce, 1);
    assert_eq!(account_b.balance, 6);
    assert_eq!(account_b.nonce, 0);
    assert!(
        state.get_account(&addr(0xe3)).is_none(),
        "intrinsic-gas failing recipient must not be created"
    );
    assert!(
        state.get_account(&addr(0xe4)).is_none(),
        "transactions after intrinsic-gas failure must not execute"
    );
}

#[test]
fn block_execution_order_controls_partial_progress_when_intrinsic_failure_is_reordered() {
    let engine = SimpleExecutionEngine::default();
    let tx_a = mk_legacy(addr(0xe5), addr(0xe6), 0, 6);
    let tx_bad = mk_eip1559(addr(0xe5), addr(0xe7), 1, 20_999, 1, vec![0xaa]);
    let tx_c = mk_blob(
        addr(0xe5),
        addr(0xe8),
        2,
        21_000,
        2,
        vec![0xbb, 0xcc],
        vec![[0x44; 32]],
    );

    let block_fail_second =
        block_with_txs_and_gas_limit(vec![tx_a.clone(), tx_bad.clone(), tx_c.clone()], 84_000);
    let block_fail_first = block_with_txs_and_gas_limit(vec![tx_bad, tx_a, tx_c], 84_000);

    let mut state_fail_second = InMemoryState::new();
    state_fail_second.upsert_account(
        addr(0xe5),
        Account {
            balance: 9,
            ..Account::default()
        },
    );
    let mut state_fail_first = state_fail_second.clone();

    let err_second = engine
        .execute_block(&mut state_fail_second, &block_fail_second)
        .expect_err("reordered intrinsic-gas failure should halt execution at index 1");
    let err_first = engine
        .execute_block(&mut state_fail_first, &block_fail_first)
        .expect_err("intrinsic-gas failure at index 0 should halt before any transfer");

    assert_eq!(
        err_second,
        ExecutionError::TxGasLimitTooLow {
            tx_gas_limit: 20_999,
            required: 21_000,
            tx_index: 1,
        }
    );
    assert_eq!(
        err_first,
        ExecutionError::TxGasLimitTooLow {
            tx_gas_limit: 20_999,
            required: 21_000,
            tx_index: 0,
        }
    );

    let sender_second = state_fail_second
        .get_account(&addr(0xe5))
        .expect("sender should reflect only first transfer in fail-second order");
    assert_eq!(sender_second.balance, 3);
    assert_eq!(sender_second.nonce, 1);
    let recipient_second = state_fail_second
        .get_account(&addr(0xe6))
        .expect("first transfer recipient should exist in fail-second order");
    assert_eq!(recipient_second.balance, 6);
    assert!(
        state_fail_second.get_account(&addr(0xe7)).is_none(),
        "intrinsic-gas failing transaction recipient must remain absent"
    );
    assert!(
        state_fail_second.get_account(&addr(0xe8)).is_none(),
        "transactions after intrinsic-gas failure must not execute"
    );

    let sender_first = state_fail_first
        .get_account(&addr(0xe5))
        .expect("sender should remain untouched when intrinsic failure is first");
    assert_eq!(sender_first.balance, 9);
    assert_eq!(sender_first.nonce, 0);
    assert!(
        state_fail_first.get_account(&addr(0xe6)).is_none(),
        "no transfer recipient should be created when first transaction fails"
    );
    assert!(
        state_fail_first.get_account(&addr(0xe7)).is_none(),
        "intrinsic-gas failing transaction recipient must remain absent"
    );
    assert!(
        state_fail_first.get_account(&addr(0xe8)).is_none(),
        "transactions after first failure must not execute"
    );
}

#[test]
fn block_execution_order_controls_contract_creation_partial_progress() {
    let engine = SimpleExecutionEngine::default();
    let contract_create = mk_legacy_contract_creation(addr(0xe9), 0, 21_000, 4, vec![0xca, 0xfe]);
    let transfer = mk_legacy(addr(0xe9), addr(0xea), 1, 7);

    let block_create_then_transfer =
        block_with_txs_and_gas_limit(vec![contract_create.clone(), transfer.clone()], 42_000);
    let block_transfer_then_create =
        block_with_txs_and_gas_limit(vec![transfer, contract_create], 42_000);

    let mut state_create_then_transfer = InMemoryState::new();
    state_create_then_transfer.upsert_account(
        addr(0xe9),
        Account {
            balance: 10,
            ..Account::default()
        },
    );
    let mut state_transfer_then_create = state_create_then_transfer.clone();

    let err_create_then_transfer = engine
        .execute_block(&mut state_create_then_transfer, &block_create_then_transfer)
        .expect_err("second transfer should fail after contract creation debits sender");
    let err_transfer_then_create = engine
        .execute_block(&mut state_transfer_then_create, &block_transfer_then_create)
        .expect_err("second contract creation should fail after transfer debits sender");

    assert_eq!(
        err_create_then_transfer,
        ExecutionError::State(StateError::InsufficientBalance {
            address: addr(0xe9),
            available: 6,
            requested: 7,
        })
    );
    assert_eq!(
        err_transfer_then_create,
        ExecutionError::State(StateError::InsufficientBalance {
            address: addr(0xe9),
            available: 3,
            requested: 4,
        })
    );

    let sender_a = state_create_then_transfer
        .get_account(&addr(0xe9))
        .expect("sender must remain present after first-order failure");
    let sender_b = state_transfer_then_create
        .get_account(&addr(0xe9))
        .expect("sender must remain present after second-order failure");
    assert_eq!(sender_a.balance, 6);
    assert_eq!(sender_a.nonce, 1);
    assert_eq!(sender_b.balance, 3);
    assert_eq!(sender_b.nonce, 1);

    assert!(
        state_create_then_transfer
            .get_account(&addr(0xea))
            .is_none(),
        "failing transfer recipient must remain absent when contract creation executes first"
    );
    let transfer_recipient = state_transfer_then_create
        .get_account(&addr(0xea))
        .expect("transfer recipient should exist when transfer executes first");
    assert_eq!(transfer_recipient.balance, 7);
}

#[test]
fn block_execution_order_controls_partial_progress_when_block_gas_limit_is_hit() {
    let engine = SimpleExecutionEngine::default();
    let tx_a = mk_legacy(addr(0xf1), addr(0xf2), 0, 7);
    let tx_b = mk_eip1559(addr(0xf1), addr(0xf3), 1, 21_000, 5, vec![0x01]);
    let tx_c = mk_blob(
        addr(0xf1),
        addr(0xf4),
        2,
        21_000,
        3,
        vec![0x02, 0x03],
        vec![[0x56; 32]],
    );

    let block_order_a =
        block_with_txs_and_gas_limit(vec![tx_a.clone(), tx_b.clone(), tx_c.clone()], 42_000);
    let block_order_b = block_with_txs_and_gas_limit(vec![tx_c, tx_b, tx_a], 42_000);

    let mut state_a = InMemoryState::new();
    state_a.upsert_account(
        addr(0xf1),
        Account {
            balance: 15,
            ..Account::default()
        },
    );
    let mut state_b = state_a.clone();

    let err_a = engine
        .execute_block(&mut state_a, &block_order_a)
        .expect_err("third transaction should exceed block gas limit in order A");
    let err_b = engine
        .execute_block(&mut state_b, &block_order_b)
        .expect_err("third transaction should exceed block gas limit in order B");

    assert_eq!(
        err_a,
        ExecutionError::GasLimitExceeded {
            gas_limit: 42_000,
            attempted: 63_000,
            tx_index: 2,
        }
    );
    assert_eq!(
        err_b,
        ExecutionError::GasLimitExceeded {
            gas_limit: 42_000,
            attempted: 63_000,
            tx_index: 2,
        }
    );

    let sender_a = state_a
        .get_account(&addr(0xf1))
        .expect("sender should retain partial progress in order A");
    let sender_b = state_b
        .get_account(&addr(0xf1))
        .expect("sender should retain partial progress in order B");
    assert_eq!(sender_a.balance, 3);
    assert_eq!(sender_a.nonce, 2);
    assert_eq!(sender_b.balance, 7);
    assert_eq!(sender_b.nonce, 2);

    let recipient_a_1 = state_a
        .get_account(&addr(0xf2))
        .expect("first recipient in order A should receive funds");
    let recipient_a_2 = state_a
        .get_account(&addr(0xf3))
        .expect("second recipient in order A should receive funds");
    assert_eq!(recipient_a_1.balance, 7);
    assert_eq!(recipient_a_2.balance, 5);
    assert!(
        state_a.get_account(&addr(0xf4)).is_none(),
        "failing third transfer recipient in order A must remain absent"
    );

    let recipient_b_1 = state_b
        .get_account(&addr(0xf4))
        .expect("first recipient in order B should receive funds");
    let recipient_b_2 = state_b
        .get_account(&addr(0xf3))
        .expect("second recipient in order B should receive funds");
    assert_eq!(recipient_b_1.balance, 3);
    assert_eq!(recipient_b_2.balance, 5);
    assert!(
        state_b.get_account(&addr(0xf2)).is_none(),
        "failing third transfer recipient in order B must remain absent"
    );
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

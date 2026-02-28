use reth2030_core::{
    Account, ExecutionEngine, ExecutionError, InMemoryState, SimpleExecutionEngine, StateError,
    StateStore,
};
use reth2030_types::{Block, Header, LegacyTx, Transaction};

fn addr(byte: u8) -> [u8; 20] {
    [byte; 20]
}

fn mk_legacy(from: [u8; 20], to: [u8; 20], nonce: u64, value: u128) -> Transaction {
    mk_legacy_with_gas(from, to, nonce, 21_000, value)
}

fn mk_legacy_with_gas(
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

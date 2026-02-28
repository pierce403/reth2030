use reth2030_core::{
    Account, ExecutionEngine, ExecutionError, InMemoryState, SimpleExecutionEngine, StateError,
    StateStore,
};
use reth2030_types::{Block, Header, LegacyTx, Transaction};

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
            gas_limit: 60_000,
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

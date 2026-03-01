use std::{
    fs,
    path::{Path, PathBuf},
};

use reth2030_core::{Account, InMemoryState, StateError, StateStore};
use reth2030_types::{BlobTx, Eip1559Tx, LegacyTx, Transaction};

const TODO_ACCEPTANCE_CRITERION_LINE: &str =
    "- [x] State backend passes deterministic transition tests.";
const REQUIRED_STATE_DETERMINISM_TESTS: [&str; 13] = [
    "fn storage_roundtrip_is_deterministic()",
    "fn apply_transactions_is_deterministic()",
    "fn zero_value_transfer_from_missing_sender_creates_accounts_deterministically()",
    "fn transfer_to_self_saturating_balance_is_deterministic_across_replays()",
    "fn apply_transactions_partial_progress_failure_is_deterministic()",
    "fn apply_transactions_mixed_variants_and_creation_is_deterministic()",
    "fn apply_transactions_mixed_variants_cross_sender_failure_is_deterministic()",
    "fn apply_transaction_contract_creation_is_deterministic()",
    "fn apply_transaction_from_missing_sender_is_deterministic_and_fail_closed()",
    "fn apply_transaction_zero_value_from_missing_sender_creates_accounts_deterministically()",
    "fn apply_transactions_zero_value_bootstrap_then_failure_is_deterministic()",
    "fn apply_transaction_nonce_and_recipient_balance_saturation_is_deterministic()",
    "fn storage_transition_sequence_is_deterministic_across_replays()",
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

#[test]
fn todo_marks_state_backend_determinism_acceptance_criterion_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines()
            .any(|line| line.trim() == TODO_ACCEPTANCE_CRITERION_LINE),
        "TODO.md must keep this acceptance criterion checked: {TODO_ACCEPTANCE_CRITERION_LINE}"
    );
}

#[test]
fn state_module_keeps_deterministic_transition_unit_test_coverage() {
    let source = read_repo_file("crates/reth2030-core/src/state.rs");

    for test_name in REQUIRED_STATE_DETERMINISM_TESTS {
        assert!(
            source.contains(test_name),
            "state.rs must keep deterministic transition coverage for `{test_name}`"
        );
    }
}

#[test]
fn deterministic_partial_progress_failure_replay_matches_error_and_post_state() {
    let txs = vec![
        Transaction::Legacy(LegacyTx {
            nonce: 0,
            from: addr(0x01),
            to: Some(addr(0x02)),
            gas_limit: 21_000,
            gas_price: 1,
            value: 8,
            data: Vec::new(),
        }),
        Transaction::Legacy(LegacyTx {
            nonce: 1,
            from: addr(0x01),
            to: Some(addr(0x03)),
            gas_limit: 21_000,
            gas_price: 1,
            value: 8,
            data: Vec::new(),
        }),
    ];

    let mut state_a = InMemoryState::new();
    state_a.upsert_account(
        addr(0x01),
        Account {
            balance: 10,
            ..Account::default()
        },
    );
    let mut state_b = state_a.clone();

    let err_a = state_a
        .apply_transactions(&txs)
        .expect_err("first run must fail");
    let err_b = state_b
        .apply_transactions(&txs)
        .expect_err("second run must fail");

    assert_eq!(
        err_a,
        StateError::InsufficientBalance {
            address: addr(0x01),
            available: 2,
            requested: 8,
        }
    );
    assert_eq!(err_a, err_b);
    assert_eq!(state_a.snapshot(), state_b.snapshot());
    assert_eq!(state_a.get_account(&addr(0x03)), None);
}

#[test]
fn deterministic_mixed_variant_cross_sender_failure_replay_halts_follow_on_transactions() {
    let txs = vec![
        Transaction::Legacy(LegacyTx {
            nonce: 0,
            from: addr(0x01),
            to: Some(addr(0x02)),
            gas_limit: 21_000,
            gas_price: 1,
            value: 3,
            data: vec![0xa1],
        }),
        Transaction::Blob(BlobTx {
            nonce: 0,
            from: addr(0x02),
            to: Some(addr(0x03)),
            gas_limit: 21_000,
            max_fee_per_gas: 3,
            max_priority_fee_per_gas: 1,
            max_fee_per_blob_gas: 2,
            value: 4,
            data: vec![0xa2],
            blob_versioned_hashes: vec![[0x88; 32]],
        }),
        Transaction::Eip1559(Eip1559Tx {
            nonce: 1,
            from: addr(0x01),
            to: Some(addr(0x04)),
            gas_limit: 21_000,
            max_fee_per_gas: 2,
            max_priority_fee_per_gas: 1,
            value: 1,
            data: vec![0xa3],
        }),
    ];

    let mut state_a = InMemoryState::new();
    state_a.upsert_account(
        addr(0x01),
        Account {
            balance: 5,
            ..Account::default()
        },
    );
    let mut state_b = state_a.clone();

    let err_a = state_a
        .apply_transactions(&txs)
        .expect_err("first run must fail");
    let err_b = state_b
        .apply_transactions(&txs)
        .expect_err("second run must fail");

    assert_eq!(
        err_a,
        StateError::InsufficientBalance {
            address: addr(0x02),
            available: 3,
            requested: 4,
        }
    );
    assert_eq!(err_a, err_b);
    assert_eq!(state_a.snapshot(), state_b.snapshot());

    let first_sender = state_a.get_account(&addr(0x01)).expect("first sender");
    let second_sender = state_a.get_account(&addr(0x02)).expect("second sender");
    assert_eq!(first_sender.balance, 2);
    assert_eq!(first_sender.nonce, 1);
    assert_eq!(second_sender.balance, 3);
    assert_eq!(second_sender.nonce, 0);
    assert_eq!(state_a.get_account(&addr(0x03)), None);
    assert_eq!(state_a.get_account(&addr(0x04)), None);
}

#[test]
fn deterministic_contract_creation_replay_mutates_only_sender() {
    let tx = Transaction::Eip1559(Eip1559Tx {
        nonce: 0,
        from: addr(0x0a),
        to: None,
        gas_limit: 21_000,
        max_fee_per_gas: 1,
        max_priority_fee_per_gas: 1,
        value: 4,
        data: vec![0xca, 0xfe],
    });

    let mut state_a = InMemoryState::new();
    state_a.upsert_account(
        addr(0x0a),
        Account {
            balance: 9,
            ..Account::default()
        },
    );
    let mut state_b = state_a.clone();

    state_a.apply_transaction(&tx).expect("first run");
    state_b.apply_transaction(&tx).expect("second run");

    assert_eq!(state_a.snapshot(), state_b.snapshot());
    assert_eq!(state_a.snapshot().len(), 1);

    let sender = state_a.get_account(&addr(0x0a)).expect("sender account");
    assert_eq!(sender.balance, 5);
    assert_eq!(sender.nonce, 1);
}

#[test]
fn deterministic_zero_value_and_storage_sequence_replay_matches_snapshots() {
    let storage_key = [0x11; 32];
    let storage_value = [0x22; 32];

    let mut state_a = InMemoryState::new();
    state_a.set_storage(addr(0xaa), storage_key, storage_value);
    state_a
        .transfer(addr(0xaa), addr(0xbb), 0)
        .expect("zero-value transfer");
    state_a.set_storage(addr(0xbb), storage_key, [0x33; 32]);

    let mut state_b = InMemoryState::new();
    state_b.set_storage(addr(0xaa), storage_key, storage_value);
    state_b
        .transfer(addr(0xaa), addr(0xbb), 0)
        .expect("zero-value transfer");
    state_b.set_storage(addr(0xbb), storage_key, [0x33; 32]);

    assert_eq!(state_a.snapshot(), state_b.snapshot());
}

#[test]
fn deterministic_apply_transaction_missing_sender_replay_is_fail_closed() {
    let tx = Transaction::Legacy(LegacyTx {
        nonce: 0,
        from: addr(0x0a),
        to: Some(addr(0x0b)),
        gas_limit: 21_000,
        gas_price: 1,
        value: 1,
        data: Vec::new(),
    });

    let mut state_a = InMemoryState::new();
    let mut state_b = InMemoryState::new();

    let err_a = state_a
        .apply_transaction(&tx)
        .expect_err("first run must fail");
    let err_b = state_b
        .apply_transaction(&tx)
        .expect_err("second run must fail");

    assert_eq!(
        err_a,
        StateError::InsufficientBalance {
            address: addr(0x0a),
            available: 0,
            requested: 1,
        }
    );
    assert_eq!(err_a, err_b);
    assert_eq!(state_a.snapshot(), state_b.snapshot());
    assert_eq!(state_a.snapshot().len(), 0);
}

#[test]
fn deterministic_apply_transaction_zero_value_missing_sender_replay_creates_accounts() {
    let tx = Transaction::Legacy(LegacyTx {
        nonce: 0,
        from: addr(0x0a),
        to: Some(addr(0x0b)),
        gas_limit: 21_000,
        gas_price: 1,
        value: 0,
        data: Vec::new(),
    });

    let mut state_a = InMemoryState::new();
    let mut state_b = InMemoryState::new();

    state_a.apply_transaction(&tx).expect("first run");
    state_b.apply_transaction(&tx).expect("second run");

    assert_eq!(state_a.snapshot(), state_b.snapshot());
    assert_eq!(state_a.snapshot().len(), 2);

    let sender = state_a.get_account(&addr(0x0a)).expect("sender account");
    let recipient = state_a.get_account(&addr(0x0b)).expect("recipient account");
    assert_eq!(sender.balance, 0);
    assert_eq!(sender.nonce, 1);
    assert_eq!(recipient.balance, 0);
    assert_eq!(recipient.nonce, 0);
}

#[test]
fn deterministic_zero_value_bootstrap_then_failure_replay_preserves_partial_progress() {
    let txs = vec![
        Transaction::Blob(BlobTx {
            nonce: 0,
            from: addr(0x0a),
            to: Some(addr(0x0b)),
            gas_limit: 21_000,
            max_fee_per_gas: 3,
            max_priority_fee_per_gas: 1,
            max_fee_per_blob_gas: 2,
            value: 0,
            data: vec![0x01, 0x02],
            blob_versioned_hashes: vec![[0x33; 32]],
        }),
        Transaction::Eip1559(Eip1559Tx {
            nonce: 1,
            from: addr(0x0a),
            to: Some(addr(0x0c)),
            gas_limit: 21_000,
            max_fee_per_gas: 2,
            max_priority_fee_per_gas: 1,
            value: 1,
            data: vec![0x03],
        }),
    ];

    let mut state_a = InMemoryState::new();
    let mut state_b = InMemoryState::new();

    let err_a = state_a
        .apply_transactions(&txs)
        .expect_err("first run must fail");
    let err_b = state_b
        .apply_transactions(&txs)
        .expect_err("second run must fail");

    assert_eq!(
        err_a,
        StateError::InsufficientBalance {
            address: addr(0x0a),
            available: 0,
            requested: 1,
        }
    );
    assert_eq!(err_a, err_b);
    assert_eq!(state_a.snapshot(), state_b.snapshot());

    let sender = state_a.get_account(&addr(0x0a)).expect("sender account");
    let first_recipient = state_a.get_account(&addr(0x0b)).expect("first recipient");
    assert_eq!(sender.balance, 0);
    assert_eq!(sender.nonce, 1);
    assert_eq!(first_recipient.balance, 0);
    assert_eq!(first_recipient.nonce, 0);
    assert_eq!(state_a.get_account(&addr(0x0c)), None);
}

#[test]
fn deterministic_apply_transaction_saturation_replay_matches_snapshot() {
    let tx = Transaction::Legacy(LegacyTx {
        nonce: 0,
        from: addr(0xaa),
        to: Some(addr(0xbb)),
        gas_limit: 21_000,
        gas_price: 1,
        value: 5,
        data: Vec::new(),
    });

    let mut state_a = InMemoryState::new();
    state_a.upsert_account(
        addr(0xaa),
        Account {
            nonce: u64::MAX,
            balance: 10,
            ..Account::default()
        },
    );
    state_a.upsert_account(
        addr(0xbb),
        Account {
            balance: u128::MAX - 2,
            ..Account::default()
        },
    );
    let mut state_b = state_a.clone();

    state_a.apply_transaction(&tx).expect("first run");
    state_b.apply_transaction(&tx).expect("second run");

    assert_eq!(state_a.snapshot(), state_b.snapshot());
    let sender = state_a.get_account(&addr(0xaa)).expect("sender account");
    let recipient = state_a.get_account(&addr(0xbb)).expect("recipient account");
    assert_eq!(sender.balance, 5);
    assert_eq!(sender.nonce, u64::MAX);
    assert_eq!(recipient.balance, u128::MAX);
}

#[test]
fn deterministic_transfer_self_saturation_replay_preserves_account_fields() {
    let storage_key = [0x51; 32];
    let storage_value = [0x52; 32];
    let mut storage = std::collections::BTreeMap::new();
    storage.insert(storage_key, storage_value);

    let initial_account = Account {
        nonce: u64::MAX,
        balance: u128::MAX,
        code: vec![0xde, 0xad],
        storage,
    };

    let mut state_a = InMemoryState::new();
    state_a.upsert_account(addr(0xaa), initial_account.clone());
    let mut state_b = InMemoryState::new();
    state_b.upsert_account(addr(0xaa), initial_account);

    state_a
        .transfer(addr(0xaa), addr(0xaa), 1)
        .expect("first run");
    state_b
        .transfer(addr(0xaa), addr(0xaa), 1)
        .expect("second run");

    assert_eq!(state_a.snapshot(), state_b.snapshot());
    let account = state_a.get_account(&addr(0xaa)).expect("sender account");
    assert_eq!(account.nonce, u64::MAX);
    assert_eq!(account.balance, u128::MAX);
    assert_eq!(account.code, vec![0xde, 0xad]);
    assert_eq!(account.storage.get(&storage_key), Some(&storage_value));
}

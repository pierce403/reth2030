use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use reth2030_core::{Account, InMemoryState, StateError, StateStore};

const TODO_SEED_TASK_LINE: &str =
    "- [x] Add state transition unit tests for basic account/storage updates.";
const REQUIRED_STATE_UNIT_TESTS: [&str; 6] = [
    "fn set_storage_creates_account_with_default_fields()",
    "fn set_storage_overwrites_per_account_without_leakage()",
    "fn set_storage_preserves_existing_account_fields_and_other_keys()",
    "fn transfer_updates_balances_and_nonce()",
    "fn transfer_error_is_atomic_for_sender_and_recipient()",
    "fn transfer_from_missing_sender_is_atomic()",
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
fn todo_marks_state_transition_seed_task_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines().any(|line| line.trim() == TODO_SEED_TASK_LINE),
        "TODO.md must keep this seed task checked: {TODO_SEED_TASK_LINE}"
    );
}

#[test]
fn state_module_keeps_basic_account_storage_transition_unit_tests() {
    let source = read_repo_file("crates/reth2030-core/src/state.rs");

    for test_name in REQUIRED_STATE_UNIT_TESTS {
        assert!(
            source.contains(test_name),
            "state.rs must keep unit test coverage for `{test_name}`"
        );
    }
}

#[test]
fn missing_sender_transfer_failure_is_fail_closed() {
    let mut state = InMemoryState::new();
    let before = state.snapshot();

    let err = state
        .transfer(addr(0xaa), addr(0xbb), 1)
        .expect_err("missing sender transfer should fail");

    assert_eq!(
        err,
        StateError::InsufficientBalance {
            address: addr(0xaa),
            available: 0,
            requested: 1,
        }
    );
    assert_eq!(state.snapshot(), before);
    assert_eq!(state.get_account(&addr(0xaa)), None);
    assert_eq!(state.get_account(&addr(0xbb)), None);
}

#[test]
fn storage_update_preserves_account_fields_and_non_target_keys() {
    let mut state = InMemoryState::new();
    let preserved_key = [0x09; 32];
    let preserved_value = [0x0a; 32];
    let target_key = [0x11; 32];
    let target_value = [0x22; 32];

    let mut storage = BTreeMap::new();
    storage.insert(preserved_key, preserved_value);
    state.upsert_account(
        addr(0x01),
        Account {
            nonce: 11,
            balance: 987,
            code: vec![0xca, 0xfe],
            storage,
        },
    );

    state.set_storage(addr(0x01), target_key, target_value);

    let account = state
        .get_account(&addr(0x01))
        .expect("account should exist");
    assert_eq!(account.nonce, 11);
    assert_eq!(account.balance, 987);
    assert_eq!(account.code, vec![0xca, 0xfe]);
    assert_eq!(account.storage.get(&preserved_key), Some(&preserved_value));
    assert_eq!(account.storage.get(&target_key), Some(&target_value));
}

#[test]
fn account_scoped_storage_writes_do_not_leak_on_shared_key() {
    let mut state = InMemoryState::new();
    let shared_key = [0x33; 32];

    state.set_storage(addr(0x01), shared_key, [0x44; 32]);
    state.set_storage(addr(0x02), shared_key, [0x55; 32]);
    state.set_storage(addr(0x01), shared_key, [0x66; 32]);

    assert_eq!(
        state.get_storage(&addr(0x01), &shared_key),
        Some([0x66; 32])
    );
    assert_eq!(
        state.get_storage(&addr(0x02), &shared_key),
        Some([0x55; 32])
    );
}

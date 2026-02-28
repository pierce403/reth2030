use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde_json::{Map, Value};

const TODO_SEED_TASK_LINE: &str = "- [x] Integrate a minimal Ethereum state-test subset first.";
const REQUIRED_FIXTURES: [(&str, &str, bool); 4] = [
    (
        "vectors/ethereum-state-tests/minimal/001-transfer-success.json",
        "transfer-success",
        true,
    ),
    (
        "vectors/ethereum-state-tests/minimal/002-insufficient-balance.json",
        "insufficient-balance",
        false,
    ),
    (
        "vectors/ethereum-state-tests/minimal/003-ordering-sensitive-failure.json",
        "ordering-sensitive-failure",
        false,
    ),
    (
        "vectors/ethereum-state-tests/minimal/nested/004-hex-transfer-success.json",
        "hex-transfer-success",
        true,
    ),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()))
}

fn collect_json_files_recursive(path: &Path) -> Vec<PathBuf> {
    fn recurse(path: &Path, files: &mut Vec<PathBuf>) {
        let mut entries: Vec<PathBuf> = fs::read_dir(path)
            .unwrap_or_else(|err| panic!("failed reading directory {}: {err}", path.display()))
            .map(|entry| {
                entry
                    .unwrap_or_else(|err| {
                        panic!(
                            "failed reading entry in directory {}: {err}",
                            path.display()
                        )
                    })
                    .path()
            })
            .collect();
        entries.sort();

        for entry_path in entries {
            if entry_path.is_dir() {
                recurse(&entry_path, files);
                continue;
            }

            if entry_path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                files.push(entry_path);
            }
        }
    }

    let mut files = Vec::new();
    recurse(path, &mut files);
    files.sort();
    files
}

fn fixture_document(relative_path: &str) -> Value {
    let contents = read_repo_file(relative_path);
    serde_json::from_str(&contents)
        .unwrap_or_else(|err| panic!("failed decoding fixture {}: {err}", relative_path))
}

fn as_object<'a>(value: &'a Value, context: &str) -> &'a Map<String, Value> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("{context} must be a JSON object"))
}

fn as_array<'a>(value: &'a Value, context: &str) -> &'a Vec<Value> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("{context} must be a JSON array"))
}

fn as_str<'a>(value: &'a Value, context: &str) -> &'a str {
    value
        .as_str()
        .unwrap_or_else(|| panic!("{context} must be a JSON string"))
}

fn as_bool(value: &Value, context: &str) -> bool {
    value
        .as_bool()
        .unwrap_or_else(|| panic!("{context} must be a JSON bool"))
}

fn parse_u128(value: &str, context: &str) -> u128 {
    let trimmed = value.trim();
    assert!(
        !trimmed.is_empty(),
        "{context} must not be an empty numeric string"
    );

    if let Some(hex_digits) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        assert!(
            !hex_digits.is_empty(),
            "{context} has invalid hex numeric string: {value}"
        );
        return u128::from_str_radix(hex_digits, 16).unwrap_or_else(|err| {
            panic!("{context} has invalid hex numeric string `{value}`: {err}")
        });
    }

    trimmed.parse::<u128>().unwrap_or_else(|err| {
        panic!("{context} has invalid decimal numeric string `{value}`: {err}")
    })
}

fn pointer_required<'a>(value: &'a Value, pointer: &str, context: &str) -> &'a Value {
    value.pointer(pointer).unwrap_or_else(|| {
        panic!("missing pointer `{pointer}` in {context}");
    })
}

fn u128_at(value: &Value, pointer: &str, context: &str) -> u128 {
    let raw = as_str(pointer_required(value, pointer, context), context);
    parse_u128(raw, &format!("{context}:{pointer}"))
}

fn normalize_repo_relative(path: &Path) -> String {
    path.strip_prefix(repo_root())
        .unwrap_or_else(|_| panic!("path {} is outside repository root", path.display()))
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn todo_marks_minimal_state_test_subset_seed_task_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines().any(|line| line.trim() == TODO_SEED_TASK_LINE),
        "TODO.md must keep the minimal state-test subset seed task checked: {TODO_SEED_TASK_LINE}"
    );
}

#[test]
fn minimal_subset_keeps_required_fixture_files_and_metadata() {
    let fixtures_dir = repo_root().join("vectors/ethereum-state-tests/minimal");
    let discovered_paths: BTreeSet<String> = collect_json_files_recursive(&fixtures_dir)
        .into_iter()
        .map(|path| normalize_repo_relative(&path))
        .collect();

    let mut discovered_names = BTreeSet::new();

    for (fixture_path, expected_name, expected_success) in REQUIRED_FIXTURES {
        assert!(
            discovered_paths.contains(fixture_path),
            "required minimal fixture path must exist: {fixture_path}"
        );

        let fixture = fixture_document(fixture_path);
        let _ = as_object(&fixture, fixture_path);
        let actual_name = as_str(
            pointer_required(&fixture, "/name", fixture_path),
            fixture_path,
        );
        let actual_success = as_bool(
            pointer_required(&fixture, "/expected/success", fixture_path),
            fixture_path,
        );

        assert_eq!(
            actual_name, expected_name,
            "fixture {fixture_path} must keep the canonical fixture name"
        );
        assert_eq!(
            actual_success, expected_success,
            "fixture {fixture_path} must keep the expected success/failure contract"
        );
        assert!(
            discovered_names.insert(actual_name.to_owned()),
            "fixture name must remain unique across the minimal subset: {actual_name}"
        );
    }

    assert!(
        discovered_paths
            .iter()
            .any(|path| path.contains("/nested/") || path.starts_with("nested/")),
        "minimal subset must keep at least one nested fixture path to lock recursive loading"
    );
}

#[test]
fn minimal_subset_preserves_mixed_outcomes_and_ordering_failure_shape() {
    let mut success_count = 0_usize;
    let mut failure_count = 0_usize;

    for (fixture_path, _name, _expected_success) in REQUIRED_FIXTURES {
        let fixture = fixture_document(fixture_path);
        if as_bool(
            pointer_required(&fixture, "/expected/success", fixture_path),
            fixture_path,
        ) {
            success_count += 1;
        } else {
            failure_count += 1;
        }
    }

    assert!(
        success_count > 0,
        "minimal subset must keep at least one successful fixture"
    );
    assert!(
        failure_count > 0,
        "minimal subset must keep at least one failing fixture"
    );

    let ordering_fixture_path =
        "vectors/ethereum-state-tests/minimal/003-ordering-sensitive-failure.json";
    let ordering_fixture = fixture_document(ordering_fixture_path);
    let transactions = as_array(
        pointer_required(&ordering_fixture, "/transactions", ordering_fixture_path),
        ordering_fixture_path,
    );
    assert!(
        transactions.len() >= 2,
        "ordering-sensitive fixture must keep at least two transactions"
    );

    let second_recipient = as_str(
        pointer_required(&transactions[1], "/to", ordering_fixture_path),
        ordering_fixture_path,
    );
    let expected_balances = as_array(
        pointer_required(
            &ordering_fixture,
            "/expected/balances",
            ordering_fixture_path,
        ),
        ordering_fixture_path,
    );

    let second_recipient_balance = expected_balances.iter().find_map(|entry| {
        let address = as_str(
            pointer_required(entry, "/address", ordering_fixture_path),
            ordering_fixture_path,
        );
        if address == second_recipient {
            Some(as_str(
                pointer_required(entry, "/balance", ordering_fixture_path),
                ordering_fixture_path,
            ))
        } else {
            None
        }
    });

    assert_eq!(
        second_recipient_balance,
        Some("0"),
        "ordering-sensitive failure must preserve a zero-balance second recipient expectation"
    );
}

#[test]
fn hex_fixture_keeps_hex_encoded_numeric_coverage() {
    let hex_fixture_path =
        "vectors/ethereum-state-tests/minimal/nested/004-hex-transfer-success.json";
    let fixture = fixture_document(hex_fixture_path);

    let initial_accounts = as_array(
        pointer_required(&fixture, "/initial_accounts", hex_fixture_path),
        hex_fixture_path,
    );
    assert!(
        !initial_accounts.is_empty(),
        "hex fixture must keep initial account rows"
    );
    assert!(
        initial_accounts.iter().all(|account| {
            as_str(
                pointer_required(account, "/balance", hex_fixture_path),
                hex_fixture_path,
            )
            .starts_with("0x")
        }),
        "hex fixture initial balances must remain hex-encoded"
    );

    let transactions = as_array(
        pointer_required(&fixture, "/transactions", hex_fixture_path),
        hex_fixture_path,
    );
    assert!(
        !transactions.is_empty(),
        "hex fixture must keep at least one transaction"
    );
    assert!(
        transactions.iter().all(|tx| {
            as_str(
                pointer_required(tx, "/value", hex_fixture_path),
                hex_fixture_path,
            )
            .starts_with("0x")
        }),
        "hex fixture transaction values must remain hex-encoded"
    );

    let expected_balances = as_array(
        pointer_required(&fixture, "/expected/balances", hex_fixture_path),
        hex_fixture_path,
    );
    assert!(
        !expected_balances.is_empty(),
        "hex fixture must keep expected balance rows"
    );
    assert!(
        expected_balances.iter().all(|entry| {
            as_str(
                pointer_required(entry, "/balance", hex_fixture_path),
                hex_fixture_path,
            )
            .starts_with("0x")
        }),
        "hex fixture expected balances must remain hex-encoded"
    );
}

#[test]
fn minimal_subset_keeps_mixed_decimal_and_hex_numeric_encodings() {
    for fixture_path in [
        "vectors/ethereum-state-tests/minimal/001-transfer-success.json",
        "vectors/ethereum-state-tests/minimal/002-insufficient-balance.json",
        "vectors/ethereum-state-tests/minimal/003-ordering-sensitive-failure.json",
    ] {
        let fixture = fixture_document(fixture_path);

        let initial_accounts = as_array(
            pointer_required(&fixture, "/initial_accounts", fixture_path),
            fixture_path,
        );
        assert!(
            initial_accounts.iter().all(|account| {
                !as_str(
                    pointer_required(account, "/balance", fixture_path),
                    fixture_path,
                )
                .starts_with("0x")
            }),
            "decimal fixtures must preserve decimal account balances: {fixture_path}"
        );

        let transactions = as_array(
            pointer_required(&fixture, "/transactions", fixture_path),
            fixture_path,
        );
        assert!(
            transactions.iter().all(|tx| {
                !as_str(pointer_required(tx, "/value", fixture_path), fixture_path)
                    .starts_with("0x")
            }),
            "decimal fixtures must preserve decimal tx values: {fixture_path}"
        );

        let expected_balances = as_array(
            pointer_required(&fixture, "/expected/balances", fixture_path),
            fixture_path,
        );
        assert!(
            expected_balances.iter().all(|entry| {
                !as_str(
                    pointer_required(entry, "/balance", fixture_path),
                    fixture_path,
                )
                .starts_with("0x")
            }),
            "decimal fixtures must preserve decimal expected balances: {fixture_path}"
        );
    }
}

#[test]
fn minimal_subset_preserves_transfer_arithmetic_and_partial_progress_invariants() {
    let transfer_fixture_path = "vectors/ethereum-state-tests/minimal/001-transfer-success.json";
    let transfer_fixture = fixture_document(transfer_fixture_path);
    let transfer_initial = u128_at(
        &transfer_fixture,
        "/initial_accounts/0/balance",
        transfer_fixture_path,
    );
    let transfer_tx_value = u128_at(
        &transfer_fixture,
        "/transactions/0/value",
        transfer_fixture_path,
    );
    let transfer_sender_expected = u128_at(
        &transfer_fixture,
        "/expected/balances/0/balance",
        transfer_fixture_path,
    );
    let transfer_recipient_expected = u128_at(
        &transfer_fixture,
        "/expected/balances/1/balance",
        transfer_fixture_path,
    );
    assert_eq!(transfer_initial, 30);
    assert_eq!(transfer_tx_value, 10);
    assert_eq!(
        transfer_sender_expected,
        transfer_initial - transfer_tx_value
    );
    assert_eq!(transfer_recipient_expected, transfer_tx_value);

    let insufficient_fixture_path =
        "vectors/ethereum-state-tests/minimal/002-insufficient-balance.json";
    let insufficient_fixture = fixture_document(insufficient_fixture_path);
    let insufficient_initial = u128_at(
        &insufficient_fixture,
        "/initial_accounts/0/balance",
        insufficient_fixture_path,
    );
    let insufficient_tx_value = u128_at(
        &insufficient_fixture,
        "/transactions/0/value",
        insufficient_fixture_path,
    );
    let insufficient_sender_expected = u128_at(
        &insufficient_fixture,
        "/expected/balances/0/balance",
        insufficient_fixture_path,
    );
    let insufficient_recipient_expected = u128_at(
        &insufficient_fixture,
        "/expected/balances/1/balance",
        insufficient_fixture_path,
    );
    assert_eq!(insufficient_initial, 5);
    assert_eq!(insufficient_tx_value, 6);
    assert!(insufficient_tx_value > insufficient_initial);
    assert_eq!(insufficient_sender_expected, insufficient_initial);
    assert_eq!(insufficient_recipient_expected, 0);

    let ordering_fixture_path =
        "vectors/ethereum-state-tests/minimal/003-ordering-sensitive-failure.json";
    let ordering_fixture = fixture_document(ordering_fixture_path);
    let ordering_initial = u128_at(
        &ordering_fixture,
        "/initial_accounts/0/balance",
        ordering_fixture_path,
    );
    let first_tx_value = u128_at(
        &ordering_fixture,
        "/transactions/0/value",
        ordering_fixture_path,
    );
    let second_tx_value = u128_at(
        &ordering_fixture,
        "/transactions/1/value",
        ordering_fixture_path,
    );
    let ordering_sender_expected = u128_at(
        &ordering_fixture,
        "/expected/balances/0/balance",
        ordering_fixture_path,
    );
    let ordering_first_recipient_expected = u128_at(
        &ordering_fixture,
        "/expected/balances/1/balance",
        ordering_fixture_path,
    );
    let ordering_second_recipient_expected = u128_at(
        &ordering_fixture,
        "/expected/balances/2/balance",
        ordering_fixture_path,
    );

    assert_eq!(ordering_initial, 30);
    assert_eq!(first_tx_value, 20);
    assert_eq!(second_tx_value, 15);
    assert!(first_tx_value <= ordering_initial);
    assert!(second_tx_value > ordering_initial - first_tx_value);
    assert_eq!(ordering_sender_expected, ordering_initial - first_tx_value);
    assert_eq!(ordering_first_recipient_expected, first_tx_value);
    assert_eq!(ordering_second_recipient_expected, 0);

    let hex_fixture_path =
        "vectors/ethereum-state-tests/minimal/nested/004-hex-transfer-success.json";
    let hex_fixture = fixture_document(hex_fixture_path);
    let hex_initial = u128_at(
        &hex_fixture,
        "/initial_accounts/0/balance",
        hex_fixture_path,
    );
    let hex_tx_value = u128_at(&hex_fixture, "/transactions/0/value", hex_fixture_path);
    let hex_sender_expected = u128_at(
        &hex_fixture,
        "/expected/balances/0/balance",
        hex_fixture_path,
    );
    let hex_recipient_expected = u128_at(
        &hex_fixture,
        "/expected/balances/1/balance",
        hex_fixture_path,
    );
    assert_eq!(hex_initial, 100);
    assert_eq!(hex_tx_value, 25);
    assert_eq!(hex_sender_expected, hex_initial - hex_tx_value);
    assert_eq!(hex_recipient_expected, hex_tx_value);
}

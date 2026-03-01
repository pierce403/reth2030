use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const TODO_SEED_TASK_LINE: &str =
    "- [x] Create `reth2030-types` crate for primitive protocol types.";
const REQUIRED_TRANSACTION_VARIANTS: [&str; 3] =
    ["Legacy(LegacyTx)", "Eip1559(Eip1559Tx)", "Blob(BlobTx)"];
const REQUIRED_PACKAGE_FIELDS: [&str; 3] = [
    "name = \"reth2030-types\"",
    "description = \"Execution-layer primitive types for reth2030\"",
    "license = \"Apache-2.0\"",
];
const REQUIRED_STRICT_SCHEMA_STRUCTS: [&str; 7] = [
    "LegacyTx",
    "Eip1559Tx",
    "BlobTx",
    "LogEntry",
    "Receipt",
    "Header",
    "Block",
];
const REQUIRED_PUBLIC_SYMBOLS: [&str; 11] = [
    "Address",
    "Hash32",
    "LegacyTx",
    "Eip1559Tx",
    "BlobTx",
    "Transaction",
    "LogEntry",
    "Receipt",
    "Header",
    "ValidationError",
    "Block",
];
const REQUIRED_EDGE_CASE_UNIT_TESTS: [&str; 12] = [
    "transaction_u128_fields_accept_max_string_inputs_for_all_variants",
    "transaction_u128_fields_reject_overflow_string_inputs",
    "transaction_u128_fields_reject_negative_integer",
    "transaction_u128_fields_reject_non_numeric_types",
    "transaction_deserialization_rejects_unknown_fields_for_all_variants",
    "transaction_deserialization_rejects_invalid_address_lengths",
    "transaction_deserialization_rejects_invalid_blob_hash_lengths",
    "block_validate_allows_empty_receipts_for_pre_execution_blocks",
    "block_validate_accepts_equal_cumulative_gas_between_receipts",
    "block_validate_rejects_non_monotonic_receipt_cumulative_gas",
    "block_validate_rejects_receipt_final_gas_below_header_gas_used",
    "block_validate_propagates_header_error_before_receipt_mismatch",
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

fn extract_public_symbols(source: &str) -> BTreeSet<String> {
    let mut symbols = BTreeSet::new();

    for line in source.lines() {
        let trimmed = line.trim_start();
        let symbol_fragment = trimmed
            .strip_prefix("pub type ")
            .or_else(|| trimmed.strip_prefix("pub struct "))
            .or_else(|| trimmed.strip_prefix("pub enum "));

        let Some(fragment) = symbol_fragment else {
            continue;
        };

        let symbol: String = fragment
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect();

        if !symbol.is_empty() {
            symbols.insert(symbol);
        }
    }

    symbols
}

fn extract_test_function_names(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut awaiting_test_fn = false;

    for line in source.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("#[test]") {
            awaiting_test_fn = true;
            continue;
        }

        if !awaiting_test_fn {
            continue;
        }

        if trimmed.starts_with("#[") || trimmed.is_empty() {
            continue;
        }

        let Some(rest) = trimmed.strip_prefix("fn ") else {
            awaiting_test_fn = false;
            continue;
        };

        let function_name: String = rest
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect();

        if !function_name.is_empty() {
            names.insert(function_name);
        }

        awaiting_test_fn = false;
    }

    names
}

#[test]
fn extract_public_symbols_captures_type_struct_and_enum_names() {
    let source = r#"
pub type Address = [u8; 20];
pub struct LegacyTx {
    value: u64,
}
pub enum Transaction {
    Legacy(LegacyTx),
}
"#;

    let symbols = extract_public_symbols(source);
    assert_eq!(
        symbols,
        BTreeSet::from([
            "Address".to_owned(),
            "LegacyTx".to_owned(),
            "Transaction".to_owned()
        ])
    );
}

#[test]
fn extract_public_symbols_ignores_non_type_public_items() {
    let source = r#"
pub(crate) type Internal = u64;
pub fn helper() {}
impl Header {
    pub fn validate_basic(&self) {}
}
pub struct Header {
    gas_limit: u64,
}
"#;

    let symbols = extract_public_symbols(source);
    assert_eq!(symbols, BTreeSet::from(["Header".to_owned()]));
}

#[test]
fn extract_test_function_names_handles_additional_attributes() {
    let source = r#"
#[test]
#[cfg(unix)]
fn unix_only_test() {}

#[test]
fn plain_test() {}
"#;

    let names = extract_test_function_names(source);
    assert_eq!(
        names,
        BTreeSet::from(["plain_test".to_owned(), "unix_only_test".to_owned()])
    );
}

#[test]
fn extract_test_function_names_ignores_non_function_declarations() {
    let source = r#"
#[test]
const NOT_A_TEST: usize = 1;
fn should_not_be_collected() {}

#[test]
fn collected_test() {}
"#;

    let names = extract_test_function_names(source);
    assert_eq!(names, BTreeSet::from(["collected_test".to_owned()]));
}

#[test]
fn todo_marks_reth2030_types_seed_task_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines().any(|line| line.trim() == TODO_SEED_TASK_LINE),
        "TODO.md must keep the `reth2030-types` seed task checked: {TODO_SEED_TASK_LINE}"
    );
}

#[test]
fn workspace_includes_reth2030_types_member() {
    let workspace_manifest = read_repo_file("Cargo.toml");
    assert!(
        workspace_manifest.contains("\"crates/reth2030-types\""),
        "workspace Cargo.toml must include `crates/reth2030-types` as a member"
    );
}

#[test]
fn reth2030_types_manifest_enforces_package_contract() {
    let manifest = read_repo_file("crates/reth2030-types/Cargo.toml");

    for required_field in REQUIRED_PACKAGE_FIELDS {
        assert!(
            manifest.lines().any(|line| line.trim() == required_field),
            "crates/reth2030-types/Cargo.toml must include `{required_field}`"
        );
    }
}

#[test]
fn reth2030_types_exports_expected_primitive_symbols_and_variants() {
    let lib = read_repo_file("crates/reth2030-types/src/lib.rs");

    let exported_symbols = extract_public_symbols(&lib);
    let expected_symbols: BTreeSet<String> = REQUIRED_PUBLIC_SYMBOLS
        .iter()
        .map(|symbol| (*symbol).to_owned())
        .collect();

    assert_eq!(
        exported_symbols, expected_symbols,
        "reth2030-types public type/struct/enum symbols drifted from the primitive protocol contract"
    );

    for variant in REQUIRED_TRANSACTION_VARIANTS {
        assert!(
            lib.contains(variant),
            "Transaction enum must include `{variant}`"
        );
    }
}

#[test]
fn reth2030_types_lib_keeps_fail_closed_serde_boundaries() {
    let lib = read_repo_file("crates/reth2030-types/src/lib.rs");
    let normalized = normalize_whitespace(&lib);

    for struct_name in REQUIRED_STRICT_SCHEMA_STRUCTS {
        let snippet = format!("#[serde(deny_unknown_fields)] pub struct {struct_name}");
        assert!(
            normalized.contains(&snippet),
            "reth2030-types `{struct_name}` must keep `#[serde(deny_unknown_fields)]`"
        );
    }
}

#[test]
fn reth2030_types_lib_keeps_edge_case_unit_coverage_contract() {
    let lib = read_repo_file("crates/reth2030-types/src/lib.rs");
    let discovered_tests = extract_test_function_names(&lib);

    for required_test in REQUIRED_EDGE_CASE_UNIT_TESTS {
        assert!(
            discovered_tests.contains(required_test),
            "crates/reth2030-types/src/lib.rs test module must include `{required_test}`"
        );
    }
}

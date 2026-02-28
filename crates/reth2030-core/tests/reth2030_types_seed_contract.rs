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

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()))
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

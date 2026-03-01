use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const TODO_SEED_TASK_LINE: &str = "- [x] Add vector harness crate or module for fixture execution.";
const REQUIRED_MANIFEST_FIELDS: [&str; 3] = [
    "name = \"reth2030-vectors\"",
    "description = \"Fixture harness and conformance scorecard tooling for reth2030\"",
    "license = \"Apache-2.0\"",
];
const REQUIRED_STRICT_SCHEMA_STRUCTS: [&str; 4] =
    ["Fixture", "FixtureBalance", "FixtureTx", "FixtureExpected"];
const REQUIRED_HARNESS_SOURCE_FRAGMENTS: [&str; 13] = [
    "#[command(name = \"reth2030-vectors\")]",
    "fn load_fixtures(fixtures_dir: &Path) -> Result<Vec<Fixture>, String>",
    "let paths = collect_fixture_paths(fixtures_dir)?;",
    "collect_fixture_paths_recursive(fixtures_dir, &mut paths)?;",
    "let metadata = fs::symlink_metadata(&path)",
    "if metadata.file_type().is_symlink() {",
    "fn execute_fixture(fixture: &Fixture) -> Result<FixtureRun, String>",
    "fn generate_reports(fixtures: &[Fixture]) -> Result<(Scorecard, SnapshotReport), String>",
    "fn compare_with_baseline(label: &str, baseline_path: &Path, generated: &str) -> Result<(), String>",
    "if fixtures.is_empty() {",
    "if failed > 0 {",
    "compare_with_baseline(\"scorecard\", &args.baseline_scorecard, &scorecard_json)?;",
    "compare_with_baseline(\"snapshot\", &args.baseline_snapshot, &snapshot_json)?;",
];
const REQUIRED_HARNESS_EDGE_CASE_TESTS: [&str; 12] = [
    "parse_u128_accepts_decimal_and_hex",
    "parse_u128_rejects_invalid_values",
    "load_fixtures_recurses_into_nested_directories",
    "load_fixtures_rejects_duplicate_fixture_names",
    "load_fixtures_rejects_unknown_fields",
    "load_fixtures_rejects_symlinked_fixture_file",
    "load_fixtures_rejects_symlinked_fixture_directory",
    "execute_fixture_accepts_hex_numeric_fields",
    "execute_fixture_flags_unexpected_post_state_accounts",
    "execute_fixture_rejects_duplicate_expected_balances",
    "compare_with_baseline_reports_line_level_diff",
    "public_minimal_suite_matches_checked_in_baseline",
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
fn extract_test_function_names_handles_additional_attributes() {
    let source = r#"
#[test]
#[cfg(unix)]
fn unix_only_123() {}

#[test]
fn simple_test() {}
"#;

    let names = extract_test_function_names(source);
    assert_eq!(
        names,
        BTreeSet::from(["simple_test".to_owned(), "unix_only_123".to_owned()])
    );
}

#[test]
fn extract_test_function_names_ignores_non_function_declarations() {
    let source = r#"
#[test]
const NOT_A_TEST: usize = 1;
fn should_not_be_captured() {}

#[test]
fn captured() {}
"#;

    let names = extract_test_function_names(source);
    assert_eq!(names, BTreeSet::from(["captured".to_owned()]));
}

#[test]
fn todo_marks_vector_harness_seed_task_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines().any(|line| line.trim() == TODO_SEED_TASK_LINE),
        "TODO.md must keep the vector harness seed task checked: {TODO_SEED_TASK_LINE}"
    );
}

#[test]
fn workspace_includes_reth2030_vectors_member() {
    let workspace_manifest = read_repo_file("Cargo.toml");
    assert!(
        workspace_manifest.contains("\"crates/reth2030-vectors\""),
        "workspace Cargo.toml must include `crates/reth2030-vectors` as a member"
    );
}

#[test]
fn reth2030_vectors_manifest_enforces_package_contract() {
    let manifest = read_repo_file("crates/reth2030-vectors/Cargo.toml");

    for required_field in REQUIRED_MANIFEST_FIELDS {
        assert!(
            manifest.lines().any(|line| line.trim() == required_field),
            "crates/reth2030-vectors/Cargo.toml must include `{required_field}`"
        );
    }
}

#[test]
fn vectors_harness_main_keeps_fixture_execution_pipeline_wiring() {
    let source = read_repo_file("crates/reth2030-vectors/src/main.rs");
    let normalized = normalize_whitespace(&source);

    for required_fragment in REQUIRED_HARNESS_SOURCE_FRAGMENTS {
        assert!(
            normalized.contains(required_fragment),
            "crates/reth2030-vectors/src/main.rs must include `{required_fragment}`"
        );
    }

    for struct_name in REQUIRED_STRICT_SCHEMA_STRUCTS {
        let strict_schema_snippet = format!("#[serde(deny_unknown_fields)] struct {struct_name}");
        assert!(
            normalized.contains(&strict_schema_snippet),
            "fixture schema type `{struct_name}` must keep fail-closed `deny_unknown_fields`"
        );
    }
}

#[test]
fn vectors_harness_main_keeps_edge_case_test_coverage_contract() {
    let source = read_repo_file("crates/reth2030-vectors/src/main.rs");
    let discovered_tests = extract_test_function_names(&source);

    for required_test in REQUIRED_HARNESS_EDGE_CASE_TESTS {
        assert!(
            discovered_tests.contains(required_test),
            "crates/reth2030-vectors/src/main.rs test module must include `{required_test}`"
        );
    }
}

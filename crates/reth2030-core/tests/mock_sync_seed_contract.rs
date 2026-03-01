use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const TODO_SEED_TASK_LINE: &str = "- [x] Node can run a mocked sync loop without panic.";
const REQUIRED_MAIN_SOURCE_SNIPPETS: [&str; 7] = [
    "run_mock_sync: bool",
    "fn run_mock_sync_once(&mut self) -> Result<(), NodeRuntimeError>",
    "fn execute(&mut self, run_mock_sync: bool) -> Result<(), NodeRuntimeError>",
    "if run_mock_sync {",
    "self.run_mock_sync_once()",
    "let shutdown_result = self.shutdown();",
    "runtime.execute(cli.run_mock_sync)",
];
const REQUIRED_RUNTIME_TESTS: [&str; 4] = [
    "mock_sync_loop_runs_without_error",
    "mock_sync_loop_fails_closed_when_no_peer_slots_are_available",
    "mock_sync_loop_can_run_repeatedly_without_panicking",
    "mock_sync_loop_retries_fail_closed_when_peer_slot_is_taken_and_recovers_when_freed",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()))
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
fn linux_only_test() {}

#[test]
fn simple_test() {}
"#;

    let names = extract_test_function_names(source);
    assert_eq!(
        names,
        BTreeSet::from(["linux_only_test".to_owned(), "simple_test".to_owned()])
    );
}

#[test]
fn extract_test_function_names_ignores_non_test_functions() {
    let source = r#"
fn helper() {}

#[test]
const SOMETHING: usize = 1;
fn should_not_be_captured() {}

#[test]
fn captured() {}
"#;

    let names = extract_test_function_names(source);
    assert_eq!(names, BTreeSet::from(["captured".to_owned()]));
}

#[test]
fn todo_marks_mock_sync_seed_task_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines().any(|line| line.trim() == TODO_SEED_TASK_LINE),
        "TODO.md must keep the mocked sync seed task checked: {TODO_SEED_TASK_LINE}"
    );
}

#[test]
fn workspace_keeps_required_mock_sync_crates() {
    let workspace_manifest = read_repo_file("Cargo.toml");
    assert!(
        workspace_manifest.contains("\"crates/reth2030\""),
        "workspace Cargo.toml must include `crates/reth2030` as a member"
    );
    assert!(
        workspace_manifest.contains("\"crates/reth2030-net\""),
        "workspace Cargo.toml must include `crates/reth2030-net` as a member"
    );
}

#[test]
fn main_keeps_mock_sync_cli_runtime_and_shutdown_wiring() {
    let main_source = read_repo_file("crates/reth2030/src/main.rs");

    for required_snippet in REQUIRED_MAIN_SOURCE_SNIPPETS {
        assert!(
            main_source.contains(required_snippet),
            "crates/reth2030/src/main.rs must include `{required_snippet}`"
        );
    }
}

#[test]
fn main_keeps_runtime_mock_sync_test_coverage_contract() {
    let main_source = read_repo_file("crates/reth2030/src/main.rs");
    let discovered_tests = extract_test_function_names(&main_source);

    for required_test in REQUIRED_RUNTIME_TESTS {
        assert!(
            discovered_tests.contains(required_test),
            "crates/reth2030/src/main.rs test module must include `{required_test}`"
        );
    }
}

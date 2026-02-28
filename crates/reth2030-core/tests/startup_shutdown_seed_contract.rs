use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const TODO_SEED_TASK_LINE: &str = "- [x] Add startup/shutdown orchestration in `reth2030` binary.";
const REQUIRED_MAIN_SOURCE_SNIPPETS: [&str; 9] = [
    "enum RuntimeState",
    "Initialized",
    "Running",
    "Stopped",
    "fn start(&mut self) -> Result<(), NodeRuntimeError>",
    "fn shutdown(&mut self) -> Result<(), NodeRuntimeError>",
    "fn execute(&mut self, run_mock_sync: bool) -> Result<(), NodeRuntimeError>",
    "let shutdown_result = self.shutdown();",
    "runtime.execute(cli.run_mock_sync)",
];
const REQUIRED_RUNTIME_TESTS: [&str; 5] = [
    "runtime_rejects_invalid_lifecycle_transitions",
    "shutdown_disconnects_all_connected_peers",
    "runtime_execute_without_mock_sync_starts_and_stops",
    "runtime_execute_with_mock_sync_success_stops_and_disconnects",
    "runtime_execute_with_mock_sync_failure_still_shuts_down",
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
fn extract_test_function_names_handles_attribute_gaps() {
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
fn todo_marks_startup_shutdown_seed_task_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines().any(|line| line.trim() == TODO_SEED_TASK_LINE),
        "TODO.md must keep the startup/shutdown seed task checked: {TODO_SEED_TASK_LINE}"
    );
}

#[test]
fn main_keeps_startup_shutdown_lifecycle_orchestration_wiring() {
    let main_source = read_repo_file("crates/reth2030/src/main.rs");

    for required_snippet in REQUIRED_MAIN_SOURCE_SNIPPETS {
        assert!(
            main_source.contains(required_snippet),
            "crates/reth2030/src/main.rs must include `{required_snippet}`"
        );
    }
}

#[test]
fn main_keeps_startup_shutdown_runtime_test_coverage_contract() {
    let main_source = read_repo_file("crates/reth2030/src/main.rs");
    let discovered_tests = extract_test_function_names(&main_source);

    for required_test in REQUIRED_RUNTIME_TESTS {
        assert!(
            discovered_tests.contains(required_test),
            "crates/reth2030/src/main.rs test module must include `{required_test}`"
        );
    }
}

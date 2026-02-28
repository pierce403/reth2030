use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_yaml::{Mapping, Sequence, Value};

const TODO_SEED_TASK_LINE: &str =
    "- [x] At least one public vector suite runs automatically in CI.";
const REQUIRED_VECTOR_JOB_ARGS: [&str; 5] = [
    "cargo run -p reth2030-vectors --locked --",
    "--fixtures-dir vectors/ethereum-state-tests/minimal",
    "--baseline-scorecard vectors/baseline/scorecard.json",
    "--baseline-snapshot vectors/baseline/snapshot.json",
    "--out-dir artifacts/vectors",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()))
}

fn workflow_document() -> Value {
    let contents = read_repo_file(".github/workflows/ci.yml");
    serde_yaml::from_str(&contents).expect("CI workflow must remain valid YAML")
}

fn as_mapping<'a>(value: &'a Value, context: &str) -> &'a Mapping {
    value
        .as_mapping()
        .unwrap_or_else(|| panic!("{context} must be a mapping"))
}

fn as_sequence<'a>(value: &'a Value, context: &str) -> &'a Sequence {
    value
        .as_sequence()
        .unwrap_or_else(|| panic!("{context} must be a sequence"))
}

fn as_str<'a>(value: &'a Value, context: &str) -> &'a str {
    value
        .as_str()
        .unwrap_or_else(|| panic!("{context} must be a string"))
}

fn map_get<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping
        .iter()
        .find_map(|(current_key, value)| match current_key {
            Value::String(current_key) if current_key == key => Some(value),
            Value::Bool(true) if key == "on" => Some(value),
            _ => None,
        })
}

fn map_get_required<'a>(mapping: &'a Mapping, key: &str, context: &str) -> &'a Value {
    map_get(mapping, key).unwrap_or_else(|| panic!("missing key `{key}` in {context}"))
}

fn find_step_by_name<'a>(steps: &'a Sequence, name: &str) -> &'a Mapping {
    steps
        .iter()
        .map(|step| as_mapping(step, "job step"))
        .find(|step| map_get(step, "name").and_then(Value::as_str) == Some(name))
        .unwrap_or_else(|| panic!("missing step named `{name}`"))
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

#[test]
fn todo_marks_public_vector_ci_seed_task_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines().any(|line| line.trim() == TODO_SEED_TASK_LINE),
        "TODO.md must keep the public vector CI seed task checked: {TODO_SEED_TASK_LINE}"
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
fn ci_workflow_still_runs_automatically_on_push_and_pull_request() {
    let doc = workflow_document();
    let root = as_mapping(&doc, "workflow root");
    let on = as_mapping(
        map_get_required(root, "on", "workflow root"),
        "workflow trigger block",
    );

    let push = as_mapping(map_get_required(on, "push", "trigger block"), "on.push");
    let branches = as_sequence(
        map_get_required(push, "branches", "push trigger"),
        "push.branches",
    );
    let branch_names: Vec<&str> = branches
        .iter()
        .map(|branch| as_str(branch, "push branch"))
        .collect();
    assert!(
        branch_names.contains(&"main"),
        "push trigger must include the main branch for automatic CI runs"
    );
    assert!(
        map_get(on, "pull_request").is_some(),
        "pull_request trigger must remain enabled for automatic CI runs"
    );
}

#[test]
fn vector_conformance_job_is_ungated_and_targets_public_minimal_suite() {
    let doc = workflow_document();
    let root = as_mapping(&doc, "workflow root");
    let jobs = as_mapping(map_get_required(root, "jobs", "workflow root"), "jobs");
    let vector_job = as_mapping(
        map_get_required(jobs, "vector-conformance", "jobs"),
        "jobs.vector-conformance",
    );

    assert!(
        map_get(vector_job, "if").is_none(),
        "vector-conformance job must not be conditionally gated"
    );

    let steps = as_sequence(
        map_get_required(vector_job, "steps", "jobs.vector-conformance"),
        "jobs.vector-conformance.steps",
    );
    let vector_run = find_step_by_name(steps, "Vector conformance");
    let vector_command = as_str(
        map_get_required(vector_run, "run", "Vector conformance step"),
        "vector run command",
    );

    for required_arg in REQUIRED_VECTOR_JOB_ARGS {
        assert!(
            vector_command.contains(required_arg),
            "vector conformance command must include `{required_arg}`"
        );
    }
}

#[test]
fn public_vector_suite_contains_json_fixtures_including_nested_cases() {
    let fixtures_dir = repo_root().join("vectors/ethereum-state-tests/minimal");
    let fixture_files = collect_json_files_recursive(&fixtures_dir);

    assert!(
        !fixture_files.is_empty(),
        "public vector fixture suite must include at least one JSON fixture"
    );
    assert!(
        fixture_files
            .iter()
            .any(|path| path.ends_with("004-hex-transfer-success.json")),
        "public vector fixture suite should keep at least one nested JSON fixture"
    );
}

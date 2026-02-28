use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_yaml::{Mapping, Sequence, Value};

fn workflow_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".github/workflows/ci.yml")
}

fn workflow_document() -> Value {
    let path = workflow_path();
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed reading CI workflow at {}: {err}", path.display()));
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

fn run_commands(steps: &Sequence) -> Vec<&str> {
    steps
        .iter()
        .filter_map(|step| {
            let step = as_mapping(step, "job step");
            map_get(step, "run").and_then(Value::as_str)
        })
        .collect()
}

fn find_step_by_uses<'a>(steps: &'a Sequence, uses: &str) -> &'a Mapping {
    steps
        .iter()
        .map(|step| as_mapping(step, "job step"))
        .find(|step| map_get(step, "uses").and_then(Value::as_str) == Some(uses))
        .unwrap_or_else(|| panic!("missing step with uses `{uses}`"))
}

fn find_step_by_name<'a>(steps: &'a Sequence, name: &str) -> &'a Mapping {
    steps
        .iter()
        .map(|step| as_mapping(step, "job step"))
        .find(|step| map_get(step, "name").and_then(Value::as_str) == Some(name))
        .unwrap_or_else(|| panic!("missing step named `{name}`"))
}

#[test]
fn ci_workflow_exists_and_parses_as_yaml() {
    let path = workflow_path();
    assert!(
        path.is_file(),
        "CI workflow file is missing at {}",
        path.display()
    );

    let doc = workflow_document();
    assert!(
        doc.as_mapping().is_some(),
        "CI workflow root must parse as a YAML mapping"
    );
}

#[test]
fn ci_workflow_triggers_push_main_and_pull_requests() {
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
        "push trigger must include the main branch"
    );

    assert!(
        map_get(on, "pull_request").is_some(),
        "pull_request trigger must be present"
    );
}

#[test]
fn rust_checks_job_runs_required_quality_gates_in_order() {
    let doc = workflow_document();
    let root = as_mapping(&doc, "workflow root");
    let jobs = as_mapping(map_get_required(root, "jobs", "workflow root"), "jobs");
    let rust_checks = as_mapping(
        map_get_required(jobs, "rust-checks", "jobs"),
        "jobs.rust-checks",
    );
    assert_eq!(
        map_get_required(rust_checks, "timeout-minutes", "jobs.rust-checks")
            .as_i64()
            .expect("timeout-minutes must be numeric"),
        30
    );

    let steps = as_sequence(
        map_get_required(rust_checks, "steps", "jobs.rust-checks"),
        "jobs.rust-checks.steps",
    );
    let commands = run_commands(steps);
    let expected = [
        "cargo fmt --all -- --check",
        "cargo check --workspace --locked",
        "cargo test --workspace --locked",
        "cargo clippy --workspace --all-targets --locked -- -D warnings",
    ];

    let mut search_start = 0usize;
    for expected_command in expected {
        let matches = commands
            .iter()
            .filter(|command| **command == expected_command)
            .count();
        assert_eq!(
            matches, 1,
            "expected exactly one `{expected_command}` command in rust-checks"
        );

        let relative_index = commands[search_start..]
            .iter()
            .position(|command| *command == expected_command)
            .unwrap_or_else(|| panic!("missing `{expected_command}` in rust-checks run steps"));
        search_start += relative_index + 1;
    }
}

#[test]
fn rust_checks_toolchain_step_requires_rustfmt_and_clippy_components() {
    let doc = workflow_document();
    let root = as_mapping(&doc, "workflow root");
    let jobs = as_mapping(map_get_required(root, "jobs", "workflow root"), "jobs");
    let rust_checks = as_mapping(
        map_get_required(jobs, "rust-checks", "jobs"),
        "jobs.rust-checks",
    );
    let steps = as_sequence(
        map_get_required(rust_checks, "steps", "jobs.rust-checks"),
        "jobs.rust-checks.steps",
    );
    let install = find_step_by_uses(steps, "dtolnay/rust-toolchain@stable");
    let with = as_mapping(
        map_get_required(install, "with", "rust toolchain install step"),
        "rust toolchain install step.with",
    );
    let components = as_sequence(
        map_get_required(with, "components", "rust toolchain install step.with"),
        "toolchain components",
    );
    let component_names: Vec<&str> = components
        .iter()
        .map(|component| as_str(component, "toolchain component"))
        .collect();
    assert!(
        component_names.contains(&"rustfmt"),
        "rustfmt component must be installed"
    );
    assert!(
        component_names.contains(&"clippy"),
        "clippy component must be installed"
    );
}

#[test]
fn ci_workflow_has_least_privilege_permissions_and_concurrency_control() {
    let doc = workflow_document();
    let root = as_mapping(&doc, "workflow root");

    let permissions = as_mapping(
        map_get_required(root, "permissions", "workflow root"),
        "permissions",
    );
    assert_eq!(
        as_str(
            map_get_required(permissions, "contents", "permissions"),
            "permissions.contents"
        ),
        "read",
        "workflow should request read-only repository contents access"
    );

    let concurrency = as_mapping(
        map_get_required(root, "concurrency", "workflow root"),
        "concurrency",
    );
    let group = as_str(
        map_get_required(concurrency, "group", "concurrency"),
        "concurrency.group",
    );
    assert!(
        group.contains("${{ github.workflow }}") && group.contains("${{ github.ref }}"),
        "concurrency group should isolate by workflow and git ref"
    );
    assert_eq!(
        map_get_required(concurrency, "cancel-in-progress", "concurrency").as_bool(),
        Some(true),
        "concurrency cancellation must be enabled to avoid stale duplicate runs"
    );
}

#[test]
fn vector_conformance_job_keeps_baseline_arguments_and_always_uploads_artifacts() {
    let doc = workflow_document();
    let root = as_mapping(&doc, "workflow root");
    let jobs = as_mapping(map_get_required(root, "jobs", "workflow root"), "jobs");
    let vector_job = as_mapping(
        map_get_required(jobs, "vector-conformance", "jobs"),
        "jobs.vector-conformance",
    );
    assert_eq!(
        map_get_required(vector_job, "timeout-minutes", "jobs.vector-conformance")
            .as_i64()
            .expect("timeout-minutes must be numeric"),
        30
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
    for required_arg in [
        "cargo run -p reth2030-vectors --locked --",
        "--fixtures-dir vectors/ethereum-state-tests/minimal",
        "--baseline-scorecard vectors/baseline/scorecard.json",
        "--baseline-snapshot vectors/baseline/snapshot.json",
        "--out-dir artifacts/vectors",
    ] {
        assert!(
            vector_command.contains(required_arg),
            "vector conformance command must include `{required_arg}`"
        );
    }

    let artifact_upload = find_step_by_uses(steps, "actions/upload-artifact@v4");
    assert_eq!(
        as_str(
            map_get_required(artifact_upload, "if", "upload-artifact step"),
            "upload-artifact if condition"
        ),
        "always()",
        "artifact upload should run even after failures"
    );

    let with = as_mapping(
        map_get_required(artifact_upload, "with", "upload-artifact step"),
        "upload-artifact step.with",
    );
    assert_eq!(
        as_str(
            map_get_required(with, "name", "upload-artifact step.with"),
            "artifact name"
        ),
        "vector-reports"
    );
    assert_eq!(
        as_str(
            map_get_required(with, "path", "upload-artifact step.with"),
            "artifact path"
        ),
        "artifacts/vectors/"
    );
    assert_eq!(
        as_str(
            map_get_required(with, "if-no-files-found", "upload-artifact step.with"),
            "artifact missing-file policy"
        ),
        "error"
    );
}

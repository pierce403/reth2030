use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const TODO_ACCEPTANCE_CRITERION_LINE: &str =
    "- [x] Peer lifecycle events are observable through logs/metrics stubs.";
const REQUIRED_PEER_SOURCE_SNIPPETS: [&str; 8] = [
    "lifecycle_logs: Vec<String>",
    "pub fn lifecycle_logs(&self) -> &[String]",
    "pub fn metrics_snapshot(&self) -> (u64, u64, u64, usize)",
    "fn record(&mut self, event: &PeerEvent, active_peers: usize)",
    "fn snapshot(&self) -> (u64, u64, u64, usize)",
    "\"peer.{} peer_id={} active_peers={}\"",
    "self.lifecycle_metrics.record(&event, self.sessions.len());",
    "Self::increment(&mut self.rejected_max_peers_total)",
];
const REQUIRED_PEER_UNIT_TESTS: [&str; 10] = [
    "connect_assigns_incrementing_session_ids_and_reconnects_in_place",
    "max_peers_limit_applies_only_to_new_peer_ids",
    "max_peers_rejection_takes_precedence_over_session_id_overflow_for_new_peer",
    "disconnect_is_idempotent_and_clears_session_state",
    "connect_fails_closed_on_session_id_overflow",
    "disconnecting_unknown_peer_does_not_emit_observability_signals",
    "clearing_events_does_not_reset_logs_or_metrics",
    "connected_metric_saturates_at_u64_max_and_keeps_active_peers_current",
    "disconnected_metric_saturates_at_u64_max_and_keeps_active_peers_current",
    "rejected_metric_saturates_at_u64_max_and_keeps_active_peers_current",
];
const REQUIRED_SYNC_TESTS: [&str; 1] = ["peer_lifecycle_events_are_observable"];
const REQUIRED_SYNC_ASSERTION_SNIPPETS: [&str; 5] = [
    "orchestrator.peer_manager.events()",
    "PeerEvent::RejectedMaxPeers(peer_id(0x02))",
    "orchestrator.peer_manager.lifecycle_logs()",
    "\"peer.rejected_max_peers peer_id=02020202020202020202020202020202 active_peers=1\"",
    "assert_eq!(orchestrator.peer_manager.metrics_snapshot(), (1, 1, 1, 0));",
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
fn extract_test_function_names_ignores_non_test_declarations() {
    let source = r#"
fn helper() {}

#[test]
const NOT_A_TEST: usize = 1;
fn ignored() {}

#[test]
fn captured() {}
"#;

    let names = extract_test_function_names(source);
    assert_eq!(names, BTreeSet::from(["captured".to_owned()]));
}

#[test]
fn todo_marks_peer_lifecycle_observability_acceptance_criterion_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines()
            .any(|line| line.trim() == TODO_ACCEPTANCE_CRITERION_LINE),
        "TODO.md must keep this acceptance criterion checked: {TODO_ACCEPTANCE_CRITERION_LINE}"
    );
}

#[test]
fn peer_manager_keeps_peer_lifecycle_observability_wiring() {
    let peer_source = read_repo_file("crates/reth2030-net/src/peer.rs");

    for required_snippet in REQUIRED_PEER_SOURCE_SNIPPETS {
        assert!(
            peer_source.contains(required_snippet),
            "crates/reth2030-net/src/peer.rs must include `{required_snippet}`"
        );
    }
}

#[test]
fn peer_manager_keeps_observability_edge_case_test_coverage_contract() {
    let peer_source = read_repo_file("crates/reth2030-net/src/peer.rs");
    let discovered_tests = extract_test_function_names(&peer_source);

    for required_test in REQUIRED_PEER_UNIT_TESTS {
        assert!(
            discovered_tests.contains(required_test),
            "crates/reth2030-net/src/peer.rs test module must include `{required_test}`"
        );
    }
}

#[test]
fn sync_orchestration_keeps_peer_observability_acceptance_coverage() {
    let sync_test_source = read_repo_file("crates/reth2030-net/tests/sync_orchestration.rs");
    let discovered_tests = extract_test_function_names(&sync_test_source);

    for required_test in REQUIRED_SYNC_TESTS {
        assert!(
            discovered_tests.contains(required_test),
            "crates/reth2030-net/tests/sync_orchestration.rs must include `{required_test}`"
        );
    }

    for required_snippet in REQUIRED_SYNC_ASSERTION_SNIPPETS {
        assert!(
            sync_test_source.contains(required_snippet),
            "crates/reth2030-net/tests/sync_orchestration.rs must include `{required_snippet}`"
        );
    }
}

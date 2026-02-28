use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const TODO_SEED_TASK_LINE: &str =
    "- [x] Create `reth2030-net` crate for peer/session abstractions.";
const REQUIRED_PACKAGE_FIELDS: [&str; 3] = [
    "name = \"reth2030-net\"",
    "description = \"Networking and sync scaffolding for reth2030\"",
    "license = \"Apache-2.0\"",
];
const REQUIRED_PUBLIC_API_SYMBOLS: [&str; 6] = [
    "PeerId",
    "PeerInfo",
    "PeerSession",
    "PeerEvent",
    "PeerManager",
    "PeerManagerError",
];
const REQUIRED_PEER_WIRING_SNIPPETS: [&str; 7] = [
    "pub type PeerId = [u8; 16];",
    "pub struct PeerInfo",
    "pub struct PeerSession",
    "pub struct PeerManager",
    "next_session_id: u64,",
    "fn allocate_session_id(&mut self) -> Result<u64, PeerManagerError>",
    ".checked_add(1)",
];
const REQUIRED_PEER_UNIT_TESTS: [&str; 7] = [
    "connect_assigns_incrementing_session_ids_and_reconnects_in_place",
    "reconnect_replaces_peer_address_and_rotates_session_id",
    "max_peers_zero_rejects_first_peer_and_keeps_state_empty",
    "max_peers_limit_applies_only_to_new_peer_ids",
    "disconnect_is_idempotent_and_clears_session_state",
    "connect_fails_closed_on_session_id_overflow",
    "disconnecting_unknown_peer_does_not_emit_observability_signals",
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
fn todo_marks_reth2030_net_seed_task_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines().any(|line| line.trim() == TODO_SEED_TASK_LINE),
        "TODO.md must keep the `reth2030-net` seed task checked: {TODO_SEED_TASK_LINE}"
    );
}

#[test]
fn workspace_includes_reth2030_net_member() {
    let workspace_manifest = read_repo_file("Cargo.toml");
    assert!(
        workspace_manifest.contains("\"crates/reth2030-net\""),
        "workspace Cargo.toml must include `crates/reth2030-net` as a member"
    );
}

#[test]
fn reth2030_net_manifest_enforces_package_contract() {
    let manifest = read_repo_file("crates/reth2030-net/Cargo.toml");

    for required_field in REQUIRED_PACKAGE_FIELDS {
        assert!(
            manifest.lines().any(|line| line.trim() == required_field),
            "crates/reth2030-net/Cargo.toml must include `{required_field}`"
        );
    }
}

#[test]
fn reth2030_net_public_api_reexports_peer_session_abstractions() {
    let lib_source = read_repo_file("crates/reth2030-net/src/lib.rs");

    for required_symbol in REQUIRED_PUBLIC_API_SYMBOLS {
        let public_api_doc_bullet = format!("//! - `{required_symbol}`:");
        assert!(
            lib_source.contains(&public_api_doc_bullet),
            "crates/reth2030-net/src/lib.rs `## Public API` docs must include `{required_symbol}`"
        );

        assert!(
            lib_source.contains(required_symbol),
            "crates/reth2030-net/src/lib.rs must re-export `{required_symbol}`"
        );
    }
}

#[test]
fn reth2030_net_peer_module_keeps_session_abstraction_wiring() {
    let peer_source = read_repo_file("crates/reth2030-net/src/peer.rs");

    for required_snippet in REQUIRED_PEER_WIRING_SNIPPETS {
        assert!(
            peer_source.contains(required_snippet),
            "crates/reth2030-net/src/peer.rs must include `{required_snippet}`"
        );
    }
}

#[test]
fn reth2030_net_peer_module_keeps_edge_case_unit_coverage_contract() {
    let peer_source = read_repo_file("crates/reth2030-net/src/peer.rs");
    let discovered_tests = extract_test_function_names(&peer_source);

    for required_test in REQUIRED_PEER_UNIT_TESTS {
        assert!(
            discovered_tests.contains(required_test),
            "crates/reth2030-net/src/peer.rs test module must include `{required_test}`"
        );
    }
}

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()))
}

fn crate_level_doc_lines(source: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut started = false;

    for line in source.lines() {
        let trimmed_start = line.trim_start();
        if let Some(doc) = trimmed_start.strip_prefix("//!") {
            lines.push(doc.trim_start());
            started = true;
            continue;
        }

        if !started && line.trim().is_empty() {
            continue;
        }

        if started && line.trim().is_empty() {
            lines.push("");
            continue;
        }

        break;
    }

    lines
}

fn public_api_section_lines<'a>(doc_lines: &'a [&'a str]) -> Option<Vec<&'a str>> {
    let mut in_public_api = false;
    let mut lines = Vec::new();

    for line in doc_lines {
        let trimmed = line.trim();
        if trimmed == "## Public API" {
            in_public_api = true;
            continue;
        }

        if in_public_api {
            if trimmed.starts_with("## ") {
                break;
            }
            lines.push(trimmed);
        }
    }

    in_public_api.then_some(lines)
}

fn parse_documented_symbols(section_lines: &[&str]) -> BTreeSet<String> {
    let mut symbols = BTreeSet::new();

    for line in section_lines {
        let trimmed = line.trim();
        if !trimmed.starts_with("- `") {
            continue;
        }

        let rest = &trimmed[3..];
        let Some(end) = rest.find('`') else {
            continue;
        };
        let symbol = &rest[..end];
        if symbol.is_empty() {
            continue;
        }

        symbols.insert(symbol.to_owned());
    }

    symbols
}

fn assert_crate_public_api_docs(relative_path: &str, expected_symbols: &[&str]) {
    let source = read_repo_file(relative_path);
    let doc_lines = crate_level_doc_lines(&source);
    assert!(
        !doc_lines.is_empty(),
        "{relative_path} must contain crate-level inner-doc comments"
    );

    let section = public_api_section_lines(&doc_lines)
        .unwrap_or_else(|| panic!("{relative_path} must contain a `## Public API` section"));
    let documented_symbols = parse_documented_symbols(&section);

    let expected: BTreeSet<String> = expected_symbols
        .iter()
        .map(|symbol| (*symbol).to_owned())
        .collect();

    assert_eq!(
        documented_symbols, expected,
        "{relative_path} `## Public API` symbols must match expected public API surface"
    );
}

#[test]
fn crate_level_doc_lines_capture_only_leading_inner_docs() {
    let source = r#"
//! Top-level docs.
//!
//! ## Public API
//! - `Foo`: documented.

pub struct Foo;

//! Not part of crate docs.
"#;

    let lines = crate_level_doc_lines(source);
    assert_eq!(
        lines,
        vec![
            "Top-level docs.",
            "",
            "## Public API",
            "- `Foo`: documented.",
            ""
        ]
    );
}

#[test]
fn public_api_section_lines_stops_at_next_h2_heading() {
    let docs = [
        "Intro",
        "## Public API",
        "- `Foo`: documented.",
        "",
        "## Notes",
        "- `Bar`: not in public-api section",
    ];

    let section = public_api_section_lines(&docs).expect("public API section should exist");
    assert_eq!(section, vec!["- `Foo`: documented.", ""]);
}

#[test]
fn parse_documented_symbols_ignores_non_bullets_and_malformed_entries() {
    let section = [
        "- `Foo`: documented",
        "- `Bar`",
        "- no backticks",
        "- ``: empty symbol",
        "text",
    ];

    let symbols = parse_documented_symbols(&section);
    assert_eq!(
        symbols,
        BTreeSet::from(["Bar".to_owned(), "Foo".to_owned()])
    );
}

#[test]
fn reth2030_core_public_api_is_documented_at_crate_level() {
    assert_crate_public_api_docs(
        "crates/reth2030-core/src/lib.rs",
        &[
            "Account",
            "BlockExecutionResult",
            "Chain",
            "ExecutionEngine",
            "ExecutionError",
            "InMemoryState",
            "NodeConfig",
            "SimpleExecutionEngine",
            "StateError",
            "StateStore",
            "StorageKey",
            "StorageValue",
            "TxExecutionResult",
        ],
    );
}

#[test]
fn reth2030_types_public_api_is_documented_at_crate_level() {
    assert_crate_public_api_docs(
        "crates/reth2030-types/src/lib.rs",
        &[
            "Address",
            "BlobTx",
            "Block",
            "Eip1559Tx",
            "Hash32",
            "Header",
            "LegacyTx",
            "LogEntry",
            "Receipt",
            "Transaction",
            "ValidationError",
        ],
    );
}

#[test]
fn reth2030_net_public_api_is_documented_at_crate_level() {
    assert_crate_public_api_docs(
        "crates/reth2030-net/src/lib.rs",
        &[
            "BlockBodyRef",
            "ExecutionSink",
            "HeaderRef",
            "MockSyncSource",
            "PeerEvent",
            "PeerId",
            "PeerInfo",
            "PeerManager",
            "PeerManagerError",
            "PeerSession",
            "RecordingExecutionSink",
            "SyncError",
            "SyncOrchestrator",
            "SyncReport",
            "SyncSource",
            "SyncStepReport",
        ],
    );
}

#[test]
fn reth2030_rpc_public_api_is_documented_at_crate_level() {
    assert_crate_public_api_docs(
        "crates/reth2030-rpc/src/lib.rs",
        &[
            "JsonRpcError",
            "JsonRpcRequest",
            "JsonRpcResponse",
            "RpcServerState",
            "router",
            "serve",
        ],
    );
}

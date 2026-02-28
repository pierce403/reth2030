use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const TODO_SEED_TASK_LINE: &str =
    "- [x] Create `reth2030-rpc` crate with HTTP JSON-RPC server skeleton.";
const REQUIRED_PACKAGE_FIELDS: [&str; 3] = [
    "name = \"reth2030-rpc\"",
    "description = \"JSON-RPC and Engine API skeleton for reth2030\"",
    "license = \"Apache-2.0\"",
];
const REQUIRED_ROUTE_PATHS: [&str; 2] = ["/", "/engine"];
const REQUIRED_ROUTER_WIRING: [&str; 2] = [
    ".route(\"/\", post(handle_rpc))",
    ".route(\"/engine\", post(handle_engine_rpc))",
];
const REQUIRED_SERVER_WIRING: [&str; 2] = [
    "let listener = tokio::net::TcpListener::bind(addr).await?;",
    "axum::serve(listener, router(state)).await",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()))
}

fn extract_route_paths(source: &str) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    let mut cursor = 0_usize;
    let needle = ".route(\"";

    while let Some(start_offset) = source[cursor..].find(needle) {
        let path_start = cursor + start_offset + needle.len();
        let Some(path_end_offset) = source[path_start..].find('"') else {
            break;
        };
        let path_end = path_start + path_end_offset;
        let route_path = &source[path_start..path_end];
        if !route_path.is_empty() {
            paths.insert(route_path.to_owned());
        }
        cursor = path_end + 1;
    }

    paths
}

#[test]
fn extract_route_paths_discovers_unique_literal_route_paths() {
    let source = r#"
Router::new()
    .route("/", post(root))
    .route("/engine", post(engine))
    .route("/", post(other_root))
"#;

    let paths = extract_route_paths(source);
    assert_eq!(
        paths,
        BTreeSet::from(["/".to_owned(), "/engine".to_owned()])
    );
}

#[test]
fn extract_route_paths_ignores_empty_or_malformed_route_literals() {
    let source = r#"
Router::new()
    .route("", post(empty))
    .route("/ok", post(ok))
    .route("/unterminated, post(bad))
"#;

    let paths = extract_route_paths(source);
    assert_eq!(paths, BTreeSet::from(["/ok".to_owned()]));
}

#[test]
fn todo_marks_reth2030_rpc_seed_task_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines().any(|line| line.trim() == TODO_SEED_TASK_LINE),
        "TODO.md must keep the `reth2030-rpc` seed task checked: {TODO_SEED_TASK_LINE}"
    );
}

#[test]
fn workspace_includes_reth2030_rpc_member() {
    let workspace_manifest = read_repo_file("Cargo.toml");
    assert!(
        workspace_manifest.contains("\"crates/reth2030-rpc\""),
        "workspace Cargo.toml must include `crates/reth2030-rpc` as a member"
    );
}

#[test]
fn reth2030_rpc_manifest_enforces_package_contract() {
    let manifest = read_repo_file("crates/reth2030-rpc/Cargo.toml");

    for required_field in REQUIRED_PACKAGE_FIELDS {
        assert!(
            manifest.lines().any(|line| line.trim() == required_field),
            "crates/reth2030-rpc/Cargo.toml must include `{required_field}`"
        );
    }
}

#[test]
fn reth2030_rpc_lib_keeps_http_json_rpc_router_and_server_skeleton() {
    let source = read_repo_file("crates/reth2030-rpc/src/lib.rs");
    let route_paths = extract_route_paths(&source);

    for required_path in REQUIRED_ROUTE_PATHS {
        assert!(
            route_paths.contains(required_path),
            "reth2030-rpc router must keep `{required_path}` route path"
        );
    }

    for required_wiring in REQUIRED_ROUTER_WIRING {
        assert!(
            source.contains(required_wiring),
            "reth2030-rpc router wiring must include `{required_wiring}`"
        );
    }

    for required_wiring in REQUIRED_SERVER_WIRING {
        assert!(
            source.contains(required_wiring),
            "reth2030-rpc server wiring must include `{required_wiring}`"
        );
    }
}

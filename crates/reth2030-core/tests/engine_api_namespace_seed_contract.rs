use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const TODO_SEED_TASK_LINE: &str =
    "- [x] Add Engine API namespace skeleton with JWT auth placeholders.";
const REQUIRED_ENGINE_WIRING_FRAGMENTS: [&str; 12] = [
    ".route(\"/engine\", post(handle_engine_rpc))",
    "const ENGINE_AUTH_ERROR_CODE: i64 = -32001;",
    "const ENGINE_NOT_IMPLEMENTED_CODE: i64 = -32004;",
    "fn handle_engine_rpc(",
    "if let Err(auth_error) = authorize_engine_request(&headers, &state.engine_jwt) {",
    "return Json(engine_auth_error_response(Value::Null, auth_error));",
    "if !is_engine_method(&request.method) {",
    "fn is_engine_method(method: &str) -> bool {",
    "method.starts_with(\"engine_\")",
    "fn authorize_engine_request(",
    "if expected_jwt.trim().is_empty() {",
    "fn engine_auth_error_response(id: Value, reason: EngineAuthError) -> JsonRpcResponse {",
];
const REQUIRED_ENGINE_AUTH_REASON_MAPPINGS: [&str; 8] = [
    "Self::MissingAuthorizationHeader => \"missing_authorization_header\"",
    "Self::MultipleAuthorizationHeaders => \"multiple_authorization_headers\"",
    "Self::InvalidAuthorizationHeaderEncoding => \"invalid_authorization_header_encoding\"",
    "Self::InvalidAuthorizationScheme => \"invalid_authorization_scheme\"",
    "Self::MissingBearerToken => \"missing_bearer_token\"",
    "Self::InvalidAuthorizationFormat => \"invalid_authorization_format\"",
    "Self::InvalidToken => \"invalid_token\"",
    "Self::MissingConfiguredJwt => \"missing_configured_jwt\"",
];
const REQUIRED_ENGINE_CAPABILITIES: [&str; 3] = [
    "engine_newPayloadV3",
    "engine_forkchoiceUpdatedV3",
    "engine_getPayloadV3",
];
const REQUIRED_ENGINE_EDGE_CASE_TESTS: [&str; 13] = [
    "engine_api_requires_authorization_header",
    "engine_api_accepts_case_insensitive_bearer_scheme",
    "engine_api_rejects_wrong_token",
    "engine_api_rejects_non_bearer_scheme",
    "engine_api_rejects_missing_bearer_token",
    "engine_api_rejects_malformed_authorization_format",
    "engine_api_rejects_duplicate_authorization_headers",
    "engine_api_rejects_non_utf8_authorization_header",
    "unauthorized_engine_requests_do_not_parse_request_body",
    "authorized_engine_requests_parse_json_after_auth",
    "engine_namespace_rejects_non_engine_methods",
    "engine_namespace_has_placeholder_method_responses",
    "engine_api_fails_closed_if_jwt_is_not_configured",
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

fn parse_test_function_name(line: &str) -> Option<String> {
    let signature = line
        .strip_prefix("fn ")
        .or_else(|| line.strip_prefix("async fn "))
        .or_else(|| line.strip_prefix("pub fn "))
        .or_else(|| line.strip_prefix("pub async fn "))?;

    let name: String = signature
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect();

    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn extract_test_function_names(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut awaiting_test_fn = false;

    for line in source.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("#[") && trimmed.contains("test") {
            awaiting_test_fn = true;
            continue;
        }

        if !awaiting_test_fn {
            continue;
        }

        if trimmed.starts_with("#[") || trimmed.is_empty() {
            continue;
        }

        if let Some(name) = parse_test_function_name(trimmed) {
            names.insert(name);
        }

        awaiting_test_fn = false;
    }

    names
}

fn extract_const_string_array_items(source: &str, const_name: &str) -> BTreeSet<String> {
    let mut items = BTreeSet::new();
    let marker = format!("const {const_name}:");
    let Some(const_start) = source.find(&marker) else {
        return items;
    };
    let source_after_const = &source[const_start..];

    let Some(initializer_offset) = source_after_const.find('=') else {
        return items;
    };
    let source_after_initializer = &source_after_const[initializer_offset + 1..];

    let Some(array_start_offset) = source_after_initializer.find('[') else {
        return items;
    };
    let source_after_array_start = &source_after_initializer[array_start_offset + 1..];

    let Some(array_end_offset) = source_after_array_start.find("];") else {
        return items;
    };
    let array_contents = &source_after_array_start[..array_end_offset];

    for entry in array_contents.split(',') {
        let trimmed = entry.trim();
        if let Some(value) = trimmed
            .strip_prefix('"')
            .and_then(|inner| inner.strip_suffix('"'))
        {
            items.insert(value.to_owned());
        }
    }

    items
}

#[test]
fn parse_test_function_name_handles_sync_and_async_signatures() {
    assert_eq!(
        parse_test_function_name("fn simple_test() {}"),
        Some("simple_test".to_owned())
    );
    assert_eq!(
        parse_test_function_name("async fn async_test() {}"),
        Some("async_test".to_owned())
    );
    assert_eq!(
        parse_test_function_name("pub async fn public_async_test() {}"),
        Some("public_async_test".to_owned())
    );
}

#[test]
fn parse_test_function_name_ignores_non_function_lines() {
    assert_eq!(parse_test_function_name("let x = 1;"), None);
    assert_eq!(parse_test_function_name("const VALUE: usize = 1;"), None);
}

#[test]
fn extract_test_function_names_supports_tokio_tests_and_extra_attributes() {
    let source = r#"
#[tokio::test]
async fn async_case() {}

#[test]
#[cfg(unix)]
fn sync_case() {}
"#;

    let names = extract_test_function_names(source);
    assert_eq!(
        names,
        BTreeSet::from(["async_case".to_owned(), "sync_case".to_owned()])
    );
}

#[test]
fn extract_const_string_array_items_reads_string_literals_only() {
    let source = r#"
const EXAMPLE: [&str; 4] = [
    "first",
    "second",
    SOME_OTHER_VALUE,
    "third",
];
"#;

    let items = extract_const_string_array_items(source, "EXAMPLE");
    assert_eq!(
        items,
        BTreeSet::from(["first".to_owned(), "second".to_owned(), "third".to_owned()])
    );
}

#[test]
fn todo_marks_engine_api_namespace_seed_task_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines().any(|line| line.trim() == TODO_SEED_TASK_LINE),
        "TODO.md must keep this seed task checked: {TODO_SEED_TASK_LINE}"
    );
}

#[test]
fn engine_rpc_source_keeps_namespace_and_jwt_placeholder_wiring() {
    let source = read_repo_file("crates/reth2030-rpc/src/lib.rs");
    let normalized = normalize_whitespace(&source);

    for required_fragment in REQUIRED_ENGINE_WIRING_FRAGMENTS {
        assert!(
            normalized.contains(required_fragment),
            "crates/reth2030-rpc/src/lib.rs must include `{required_fragment}`"
        );
    }

    for required_mapping in REQUIRED_ENGINE_AUTH_REASON_MAPPINGS {
        assert!(
            normalized.contains(required_mapping),
            "EngineAuthError::reason mapping must include `{required_mapping}`"
        );
    }
}

#[test]
fn engine_capabilities_constant_keeps_expected_placeholder_methods() {
    let source = read_repo_file("crates/reth2030-rpc/src/lib.rs");
    let capabilities = extract_const_string_array_items(&source, "ENGINE_CAPABILITIES");

    assert_eq!(
        capabilities,
        BTreeSet::from(REQUIRED_ENGINE_CAPABILITIES.map(str::to_owned)),
        "ENGINE_CAPABILITIES must remain stable for placeholder Engine API namespace support"
    );
}

#[test]
fn engine_rpc_source_keeps_auth_and_namespace_edge_case_test_coverage_contract() {
    let source = read_repo_file("crates/reth2030-rpc/src/lib.rs");
    let discovered_tests = extract_test_function_names(&source);

    for required_test in REQUIRED_ENGINE_EDGE_CASE_TESTS {
        assert!(
            discovered_tests.contains(required_test),
            "crates/reth2030-rpc/src/lib.rs test module must include `{required_test}`"
        );
    }
}

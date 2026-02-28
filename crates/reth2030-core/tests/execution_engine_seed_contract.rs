use std::{
    fs,
    path::{Path, PathBuf},
};

use reth2030_core::{ExecutionEngine, InMemoryState, SimpleExecutionEngine};
use reth2030_types::{Block, Header};

const TODO_SEED_TASK_LINE: &str = "- [x] Define an `ExecutionEngine` trait in core.";
const REQUIRED_TRAIT_SIGNATURE_FRAGMENTS: [&str; 5] = [
    "fn execute_block(",
    "&self,",
    "state: &mut dyn StateStore,",
    "block: &Block,",
    "-> Result<BlockExecutionResult, ExecutionError>;",
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

fn extract_trait_body<'a>(source: &'a str, trait_name: &str) -> Option<&'a str> {
    let marker = format!("trait {trait_name}");
    let trait_start = source.find(&marker)?;
    let after_trait = &source[trait_start..];
    let open_offset = after_trait.find('{')?;
    let body_start = trait_start + open_offset + 1;

    let bytes = source.as_bytes();
    let mut depth = 1_usize;
    let mut index = body_start;

    while index < bytes.len() {
        match bytes[index] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&source[body_start..index]);
                }
            }
            _ => {}
        }
        index += 1;
    }

    None
}

fn empty_block() -> Block {
    Block {
        header: Header {
            parent_hash: [0; 32],
            number: 1,
            timestamp: 1_762_312_000,
            gas_limit: 30_000_000,
            gas_used: 0,
            state_root: [0; 32],
            transactions_root: [0; 32],
            receipts_root: [0; 32],
        },
        transactions: Vec::new(),
        receipts: Vec::new(),
    }
}

#[test]
fn normalize_whitespace_collapses_spacing_and_line_breaks() {
    let input = "fn execute_block(\n   &self,\tstate: &mut dyn StateStore,\n)";
    let normalized = normalize_whitespace(input);
    assert_eq!(
        normalized,
        "fn execute_block( &self, state: &mut dyn StateStore, )"
    );
}

#[test]
fn extract_trait_body_handles_nested_default_method_blocks() {
    let source = r#"
pub trait Unrelated {
    fn noop(&self);
}

pub trait ExecutionEngine {
    fn execute_block(&self);
    fn helper(&self) {
        if true {
            let _ = 1;
        }
    }
}
"#;

    let body = extract_trait_body(source, "ExecutionEngine").expect("trait should be extracted");
    assert!(body.contains("fn execute_block(&self);"));
    assert!(body.contains("fn helper(&self) {"));
}

#[test]
fn extract_trait_body_returns_none_for_unclosed_trait() {
    let source = "pub trait ExecutionEngine { fn execute_block(&self);";
    assert_eq!(extract_trait_body(source, "ExecutionEngine"), None);
}

#[test]
fn todo_marks_execution_engine_seed_task_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines().any(|line| line.trim() == TODO_SEED_TASK_LINE),
        "TODO.md must keep the execution-engine seed task checked: {TODO_SEED_TASK_LINE}"
    );
}

#[test]
fn reth2030_core_execution_module_keeps_object_safe_execution_engine_signature() {
    let source = read_repo_file("crates/reth2030-core/src/execution.rs");
    let trait_body = extract_trait_body(&source, "ExecutionEngine")
        .expect("execution.rs must define `ExecutionEngine` trait");
    let normalized_body = normalize_whitespace(trait_body);

    for fragment in REQUIRED_TRAIT_SIGNATURE_FRAGMENTS {
        assert!(
            normalized_body.contains(fragment),
            "ExecutionEngine trait signature must include `{fragment}`"
        );
    }

    assert!(
        !normalized_body.contains("state: &mut S"),
        "ExecutionEngine should avoid generic state type params in its method signature"
    );
}

#[test]
fn reth2030_core_public_api_keeps_execution_engine_reexport() {
    let source = read_repo_file("crates/reth2030-core/src/lib.rs");
    assert!(
        source.contains("ExecutionEngine"),
        "crates/reth2030-core/src/lib.rs must re-export `ExecutionEngine`"
    );
    assert!(
        source.contains("SimpleExecutionEngine"),
        "crates/reth2030-core/src/lib.rs must re-export `SimpleExecutionEngine`"
    );
}

#[test]
fn execution_engine_trait_is_dyn_dispatchable_from_public_api() {
    let engine: Box<dyn ExecutionEngine> = Box::new(SimpleExecutionEngine::default());
    let mut state = InMemoryState::new();
    let block = empty_block();

    let result = engine
        .execute_block(&mut state, &block)
        .expect("empty block should execute through dyn-dispatched engine");

    assert_eq!(result.total_gas_used, 0);
    assert!(result.tx_results.is_empty());
    assert!(result.receipts.is_empty());
}

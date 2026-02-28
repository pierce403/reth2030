use std::{
    fs,
    path::{Path, PathBuf},
};

const CORE_CHECK_COMMANDS: [&str; 4] = [
    "cargo fmt --all -- --check",
    "cargo check --workspace",
    "cargo test --workspace",
    "cargo clippy --workspace --all-targets -- -D warnings",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()))
}

fn fenced_code_blocks(markdown: &str) -> Vec<Vec<String>> {
    let mut blocks = Vec::new();
    let mut current_block: Option<Vec<String>> = None;

    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            if let Some(block) = current_block.take() {
                blocks.push(block);
            } else {
                current_block = Some(Vec::new());
            }
            continue;
        }

        if let Some(block) = current_block.as_mut() {
            block.push(line.to_owned());
        }
    }

    blocks
}

fn non_empty_trimmed_lines(block: &[String]) -> Vec<&str> {
    block
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect()
}

fn block_starts_with_core_checks(block: &[String]) -> bool {
    let lines = non_empty_trimmed_lines(block);
    lines.starts_with(&CORE_CHECK_COMMANDS)
}

fn count_blocks_starting_with_core_checks(markdown: &str) -> usize {
    fenced_code_blocks(markdown)
        .into_iter()
        .filter(|block| block_starts_with_core_checks(block))
        .count()
}

#[test]
fn fenced_code_blocks_ignore_unclosed_fences_and_keep_closed_blocks() {
    let markdown = r#"
outside
```bash
cargo fmt --all -- --check
```
```txt
line-a
line-b
```
```rust
fn this_block_is_unclosed() {}
"#;

    let blocks = fenced_code_blocks(markdown);
    assert_eq!(
        blocks.len(),
        2,
        "only fully closed fenced blocks should be parsed"
    );
    assert_eq!(
        non_empty_trimmed_lines(&blocks[0]),
        vec!["cargo fmt --all -- --check"],
        "language-tagged fenced blocks should retain command lines"
    );
    assert_eq!(
        non_empty_trimmed_lines(&blocks[1]),
        vec!["line-a", "line-b"],
        "multi-line fenced blocks should preserve line ordering"
    );
}

#[test]
fn block_starts_with_core_checks_requires_exact_prefix_order() {
    let valid_with_extra = vec![
        "cargo fmt --all -- --check".to_owned(),
        "cargo check --workspace".to_owned(),
        "cargo test --workspace".to_owned(),
        "cargo clippy --workspace --all-targets -- -D warnings".to_owned(),
        "cargo run -p reth2030 -- --help".to_owned(),
    ];
    assert!(
        block_starts_with_core_checks(&valid_with_extra),
        "extra commands after the core check prefix should remain valid"
    );

    let missing_prefix = vec![
        "cargo check --workspace".to_owned(),
        "cargo fmt --all -- --check".to_owned(),
        "cargo test --workspace".to_owned(),
        "cargo clippy --workspace --all-targets -- -D warnings".to_owned(),
    ];
    assert!(
        !block_starts_with_core_checks(&missing_prefix),
        "out-of-order commands must fail prefix validation"
    );

    let missing_command = vec![
        "cargo fmt --all -- --check".to_owned(),
        "cargo check --workspace".to_owned(),
        "cargo clippy --workspace --all-targets -- -D warnings".to_owned(),
    ];
    assert!(
        !block_starts_with_core_checks(&missing_command),
        "incomplete command lists must fail prefix validation"
    );
}

#[test]
fn count_blocks_starting_with_core_checks_counts_only_prefix_matches() {
    let markdown = r#"
```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p reth2030 -- --help
```
"#;

    assert_eq!(
        count_blocks_starting_with_core_checks(markdown),
        2,
        "only blocks with the exact core-check prefix should count"
    );
}

#[test]
fn contributing_documents_one_explicit_core_check_block() {
    let contents = read_repo_file("CONTRIBUTING.md");
    let exact_matches = fenced_code_blocks(&contents)
        .iter()
        .filter(|block| non_empty_trimmed_lines(block).as_slice() == CORE_CHECK_COMMANDS)
        .count();
    assert_eq!(
        exact_matches, 1,
        "CONTRIBUTING.md must contain exactly one fenced block listing only the four core checks"
    );
}

#[test]
fn readme_quick_start_has_single_core_check_prefix_block() {
    let contents = read_repo_file("README.md");
    let prefix_matches = count_blocks_starting_with_core_checks(&contents);
    assert_eq!(
        prefix_matches, 1,
        "README.md must include exactly one fenced block that starts with the four core checks"
    );
}

#[test]
fn agents_build_commands_has_single_core_check_prefix_block() {
    let contents = read_repo_file("AGENTS.md");
    let prefix_matches = count_blocks_starting_with_core_checks(&contents);
    assert_eq!(
        prefix_matches, 1,
        "AGENTS.md must include exactly one fenced block that starts with the four core checks"
    );
}

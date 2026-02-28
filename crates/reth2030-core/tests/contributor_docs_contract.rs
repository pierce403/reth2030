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

fn command_sequence_exists_in_order(lines: &[&str], commands: &[&str]) -> bool {
    let mut search_start = 0usize;
    for expected in commands {
        let Some(relative_index) = lines[search_start..]
            .iter()
            .position(|line| line == expected)
        else {
            return false;
        };
        search_start += relative_index + 1;
    }

    true
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
fn contributing_documents_one_explicit_core_check_block() {
    let contents = read_repo_file("CONTRIBUTING.md");
    let blocks = fenced_code_blocks(&contents);
    let exact_matches = blocks
        .iter()
        .filter(|block| non_empty_trimmed_lines(block).as_slice() == CORE_CHECK_COMMANDS)
        .count();
    assert_eq!(
        exact_matches, 1,
        "CONTRIBUTING.md must contain exactly one fenced block listing the four core checks"
    );

    for command in CORE_CHECK_COMMANDS {
        assert!(
            contents.matches(command).count() >= 1,
            "CONTRIBUTING.md is missing required core check command `{command}`"
        );
    }
}

#[test]
fn readme_quick_start_includes_all_core_checks_in_order() {
    let contents = read_repo_file("README.md");
    let blocks = fenced_code_blocks(&contents);
    let in_order_matches = blocks
        .iter()
        .filter(|block| {
            command_sequence_exists_in_order(&non_empty_trimmed_lines(block), &CORE_CHECK_COMMANDS)
        })
        .count();
    assert!(
        in_order_matches > 0,
        "README.md must include a Quick Start fenced block that lists core checks in order"
    );

    for command in CORE_CHECK_COMMANDS {
        assert!(
            contents.contains(command),
            "README.md is missing required core check command `{command}`"
        );
    }
}

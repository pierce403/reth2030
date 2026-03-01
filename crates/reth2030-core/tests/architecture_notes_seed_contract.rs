use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const TODO_SEED_TASK_LINE: &str = "- [x] Add architecture-notes directory for ADR-style decisions.";
const BOOTSTRAP_ADR_FILE: &str = "ADR-0001-initial-workspace-and-poc-scope.md";
const REQUIRED_ADR_CONTRACT_TESTS: [&str; 8] = [
    "parse_adr_filename_accepts_only_strict_adr_convention",
    "validate_adr_sequence_rejects_empty_duplicates_and_gaps",
    "is_valid_iso_date_enforces_calendar_boundaries",
    "ensure_not_symlink_accepts_regular_paths_and_rejects_missing_path",
    "ensure_not_symlink_rejects_symlink_path",
    "validate_required_sections_rejects_missing_duplicate_order_and_empty_bodies",
    "validate_adr_content_contract_rejects_invalid_title_status_and_date",
    "architecture_notes_directory_contains_well_formed_ordered_adrs",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()))
}

fn contains_checked_task_line(markdown: &str, expected_task_line: &str) -> bool {
    markdown
        .lines()
        .map(str::trim)
        .any(|line| line == expected_task_line)
}

fn parse_adr_filename(file_name: &str) -> Option<u32> {
    let after_prefix = file_name.strip_prefix("ADR-")?;
    let (number, slug_with_ext) = after_prefix.split_once('-')?;
    if number.len() != 4 || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let slug = slug_with_ext.strip_suffix(".md")?;
    if slug.is_empty() {
        return None;
    }
    if slug.starts_with('-') || slug.ends_with('-') || slug.contains("--") {
        return None;
    }
    if !slug
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return None;
    }

    number.parse::<u32>().ok()
}

fn parse_test_function_name(line: &str) -> Option<String> {
    let signature = line
        .strip_prefix("fn ")
        .or_else(|| line.strip_prefix("async fn "))
        .or_else(|| line.strip_prefix("pub fn "))
        .or_else(|| line.strip_prefix("pub async fn "))?;

    let name: String = signature
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
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

fn adr_markdown_files(dir: &Path) -> Vec<(String, u32)> {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|err| panic!("failed reading {}: {err}", dir.display()));
    let mut files = Vec::new();

    for entry in entries {
        let entry = entry.unwrap_or_else(|err| panic!("failed reading directory entry: {err}"));
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| panic!("invalid UTF-8 file name at {}", path.display()));
        let number = parse_adr_filename(file_name).unwrap_or_else(|| {
            panic!("ADR markdown files must follow strict naming; found {file_name}")
        });
        files.push((file_name.to_owned(), number));
    }

    files.sort_unstable_by_key(|(_, number)| *number);
    files
}

fn validate_contiguous_from_one(numbers: &[u32]) -> Result<(), String> {
    if numbers.is_empty() {
        return Err("at least one ADR markdown file is required".to_owned());
    }
    if numbers[0] != 1 {
        return Err(format!(
            "ADR numbering must start at 0001, found ADR-{:04}",
            numbers[0]
        ));
    }

    for pair in numbers.windows(2) {
        let previous = pair[0];
        let current = pair[1];
        if current == previous {
            return Err(format!("duplicate ADR number ADR-{current:04}"));
        }
        if current != previous + 1 {
            return Err(format!(
                "ADR numbering gap between ADR-{previous:04} and ADR-{current:04}"
            ));
        }
    }

    Ok(())
}

#[test]
fn contains_checked_task_line_matches_trimmed_checked_items_only() {
    let markdown = "\
      - [x] Keep this checked
- [ ] Keep this unchecked
- [x] Add architecture-notes directory for ADR-style decisions.
- [x] Add architecture-notes directory for ADR-style decisions. (duplicate context)
";
    assert!(contains_checked_task_line(markdown, TODO_SEED_TASK_LINE));
    assert!(!contains_checked_task_line(
        markdown,
        "- [ ] Add architecture-notes directory for ADR-style decisions."
    ));
}

#[test]
fn parse_adr_filename_accepts_and_rejects_expected_forms() {
    assert_eq!(parse_adr_filename("ADR-0001-initial-scope.md"), Some(1));
    assert_eq!(parse_adr_filename("ADR-0042-eip4844-notes.md"), Some(42));

    for invalid in [
        "ADR-1-initial-scope.md",
        "ADR-0001-initial_scope.md",
        "ADR-0001-Initial-scope.md",
        "ADR-0001-.md",
        "ADR-0001-initial--scope.md",
        "ADR-0001-initial-scope.markdown",
        "adr-0001-initial-scope.md",
        "ADR-000a-initial-scope.md",
    ] {
        assert_eq!(
            parse_adr_filename(invalid),
            None,
            "{invalid} should be rejected"
        );
    }
}

#[test]
fn parse_test_function_name_supports_sync_async_and_public_signatures() {
    assert_eq!(
        parse_test_function_name("fn local_test() {}"),
        Some("local_test".to_owned())
    );
    assert_eq!(
        parse_test_function_name("async fn async_case() {}"),
        Some("async_case".to_owned())
    );
    assert_eq!(
        parse_test_function_name("pub async fn exported_case() {}"),
        Some("exported_case".to_owned())
    );
    assert_eq!(parse_test_function_name("let value = 1;"), None);
}

#[test]
fn extract_test_function_names_handles_attribute_stacks() {
    let source = r#"
#[test]
fn one() {}

#[tokio::test]
#[cfg(unix)]
async fn two() {}

fn helper() {}
"#;
    let names = extract_test_function_names(source);
    assert_eq!(names, BTreeSet::from(["one".to_owned(), "two".to_owned()]));
}

#[test]
fn validate_contiguous_from_one_rejects_empty_duplicates_and_gaps() {
    assert_eq!(
        validate_contiguous_from_one(&[]),
        Err("at least one ADR markdown file is required".to_owned())
    );
    assert_eq!(
        validate_contiguous_from_one(&[2]),
        Err("ADR numbering must start at 0001, found ADR-0002".to_owned())
    );
    assert_eq!(
        validate_contiguous_from_one(&[1, 1]),
        Err("duplicate ADR number ADR-0001".to_owned())
    );
    assert_eq!(
        validate_contiguous_from_one(&[1, 3]),
        Err("ADR numbering gap between ADR-0001 and ADR-0003".to_owned())
    );
    assert_eq!(validate_contiguous_from_one(&[1, 2, 3]), Ok(()));
}

#[test]
fn todo_marks_architecture_notes_seed_task_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        contains_checked_task_line(&todo, TODO_SEED_TASK_LINE),
        "TODO.md must keep the architecture-notes seed task checked: {TODO_SEED_TASK_LINE}"
    );
}

#[test]
fn architecture_notes_directory_keeps_bootstrap_adr_and_strict_numbering() {
    let dir = repo_root().join("docs/architecture-notes");
    assert!(
        dir.exists() && dir.is_dir(),
        "{} must exist as a directory for ADR notes",
        dir.display()
    );

    let files = adr_markdown_files(&dir);
    assert!(
        !files.is_empty(),
        "{} must contain at least one ADR markdown file",
        dir.display()
    );

    let file_names = files
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    assert!(
        file_names.contains(BOOTSTRAP_ADR_FILE),
        "docs/architecture-notes must retain bootstrap ADR `{BOOTSTRAP_ADR_FILE}`"
    );

    let numbers = files.iter().map(|(_, number)| *number).collect::<Vec<_>>();
    validate_contiguous_from_one(&numbers)
        .unwrap_or_else(|err| panic!("invalid ADR numbering in {}: {err}", dir.display()));
}

#[test]
fn architecture_notes_contract_keeps_required_validation_tests() {
    let source = read_repo_file("crates/reth2030-core/tests/architecture_notes_contract.rs");
    let test_names = extract_test_function_names(&source);

    for test_name in REQUIRED_ADR_CONTRACT_TESTS {
        assert!(
            test_names.contains(test_name),
            "architecture_notes_contract.rs must keep test `{test_name}`"
        );
    }
}

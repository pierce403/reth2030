use std::{
    fs,
    path::{Path, PathBuf},
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn architecture_notes_dir() -> PathBuf {
    repo_root().join("docs/architecture-notes")
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
    if !slug
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return None;
    }
    if slug.starts_with('-') || slug.ends_with('-') || slug.contains("--") {
        return None;
    }

    number.parse::<u32>().ok()
}

fn validate_adr_sequence(numbers: &[u32]) -> Result<(), String> {
    if numbers.is_empty() {
        return Err("at least one ADR markdown file is required".to_owned());
    }

    let mut ordered = numbers.to_vec();
    ordered.sort_unstable();
    if ordered[0] != 1 {
        return Err(format!(
            "ADR numbering must start at 0001, found ADR-{:04}",
            ordered[0]
        ));
    }

    for pair in ordered.windows(2) {
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

fn assert_adr_content_contract(path: &Path, number: u32, content: &str) {
    let expected_title_prefix = format!("# ADR-{number:04}:");
    assert!(
        content
            .lines()
            .any(|line| line.trim_start().starts_with(&expected_title_prefix)),
        "{} must include a title heading starting with `{expected_title_prefix}`",
        path.display()
    );

    for required_marker in [
        "- Status:",
        "- Date:",
        "## Context",
        "## Decision",
        "## Consequences",
    ] {
        assert!(
            content.contains(required_marker),
            "{} is missing required marker `{required_marker}`",
            path.display()
        );
    }
}

#[test]
fn parse_adr_filename_accepts_only_strict_adr_convention() {
    assert_eq!(parse_adr_filename("ADR-0001-initial-scope.md"), Some(1));
    assert_eq!(parse_adr_filename("ADR-0420-example-2026.md"), Some(420));

    for invalid in [
        "adr-0001-initial-scope.md",
        "ADR-1-initial-scope.md",
        "ADR-0001.md",
        "ADR-0001-.md",
        "ADR-0001-Initial-scope.md",
        "ADR-0001-initial_scope.md",
        "ADR-00a1-initial-scope.md",
        "ADR-0001-initial-scope.markdown",
        "ADR-0001-initial--scope.md",
    ] {
        assert_eq!(
            parse_adr_filename(invalid),
            None,
            "{invalid} should be rejected"
        );
    }
}

#[test]
fn validate_adr_sequence_rejects_empty_duplicates_and_gaps() {
    assert_eq!(
        validate_adr_sequence(&[]),
        Err("at least one ADR markdown file is required".to_owned())
    );
    assert_eq!(
        validate_adr_sequence(&[2]),
        Err("ADR numbering must start at 0001, found ADR-0002".to_owned())
    );
    assert_eq!(
        validate_adr_sequence(&[1, 1]),
        Err("duplicate ADR number ADR-0001".to_owned())
    );
    assert_eq!(
        validate_adr_sequence(&[1, 3]),
        Err("ADR numbering gap between ADR-0001 and ADR-0003".to_owned())
    );

    assert_eq!(validate_adr_sequence(&[3, 1, 2]), Ok(()));
}

#[test]
fn architecture_notes_directory_contains_well_formed_ordered_adrs() {
    let dir = architecture_notes_dir();
    assert!(
        dir.exists() && dir.is_dir(),
        "{} must exist as a directory for ADR documents",
        dir.display()
    );

    let entries =
        fs::read_dir(&dir).unwrap_or_else(|err| panic!("failed reading {}: {err}", dir.display()));
    let mut numbers = Vec::new();
    let mut adr_paths = Vec::new();
    let mut unexpected_markdown_files = Vec::new();

    for entry in entries {
        let entry = entry.unwrap_or_else(|err| panic!("failed reading entry: {err}"));
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
        if let Some(number) = parse_adr_filename(file_name) {
            numbers.push(number);
            adr_paths.push((number, path));
        } else {
            unexpected_markdown_files.push(file_name.to_owned());
        }
    }

    assert!(
        unexpected_markdown_files.is_empty(),
        "docs/architecture-notes should only include ADR markdown files, found: {:?}",
        unexpected_markdown_files
    );
    validate_adr_sequence(&numbers)
        .unwrap_or_else(|err| panic!("invalid ADR numbering in {}: {err}", dir.display()));

    adr_paths.sort_unstable_by_key(|(number, _)| *number);
    for (number, path) in adr_paths {
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()));
        assert_adr_content_contract(&path, number, &content);
    }
}

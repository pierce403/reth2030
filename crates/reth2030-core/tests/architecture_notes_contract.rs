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

const ALLOWED_ADR_STATUSES: [&str; 5] = [
    "Proposed",
    "Accepted",
    "Superseded",
    "Rejected",
    "Deprecated",
];
const REQUIRED_ADR_SECTIONS: [&str; 3] = ["## Context", "## Decision", "## Consequences"];

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

fn parse_single_metadata_value(content: &str, key: &str) -> Result<String, String> {
    let prefix = format!("- {key}:");
    let mut values = content
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix(&prefix).map(str::trim));

    let value = values
        .next()
        .ok_or_else(|| format!("missing required metadata line `{prefix}`"))?;
    if value.is_empty() {
        return Err(format!("metadata line `{prefix}` must not be empty"));
    }
    if values.next().is_some() {
        return Err(format!("metadata line `{prefix}` must appear exactly once"));
    }

    Ok(value.to_owned())
}

fn is_valid_iso_date(date: &str) -> bool {
    if date.len() != 10 {
        return false;
    }
    let bytes = date.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }

    let year = date[0..4].parse::<u32>().ok();
    let month = date[5..7].parse::<u32>().ok();
    let day = date[8..10].parse::<u32>().ok();
    let (year, month, day) = match (year, month, day) {
        (Some(year), Some(month), Some(day)) => (year, month, day),
        _ => return false,
    };

    if month == 0 || month > 12 || day == 0 {
        return false;
    }

    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let is_leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
            if is_leap_year {
                29
            } else {
                28
            }
        }
        _ => return false,
    };

    day <= max_day
}

fn validate_required_sections(content: &str) -> Result<(), String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut section_positions = Vec::with_capacity(REQUIRED_ADR_SECTIONS.len());

    for heading in REQUIRED_ADR_SECTIONS {
        let positions: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| (line.trim() == heading).then_some(index))
            .collect();

        if positions.is_empty() {
            return Err(format!("missing required heading `{heading}`"));
        }
        if positions.len() > 1 {
            return Err(format!(
                "required heading `{heading}` must appear exactly once"
            ));
        }
        section_positions.push((heading, positions[0]));
    }

    for window in section_positions.windows(2) {
        let (current_heading, current_pos) = window[0];
        let (next_heading, next_pos) = window[1];
        if current_pos >= next_pos {
            return Err(format!(
                "required heading `{current_heading}` must appear before `{next_heading}`"
            ));
        }
        if !lines[current_pos + 1..next_pos]
            .iter()
            .any(|line| !line.trim().is_empty())
        {
            return Err(format!(
                "section `{current_heading}` must contain at least one non-empty line"
            ));
        }
    }

    let (last_heading, last_pos) = section_positions
        .last()
        .copied()
        .expect("required section headings are non-empty");
    if !lines[last_pos + 1..]
        .iter()
        .any(|line| !line.trim().is_empty())
    {
        return Err(format!(
            "section `{last_heading}` must contain at least one non-empty line"
        ));
    }

    Ok(())
}

fn validate_adr_content_contract(number: u32, content: &str) -> Result<(), String> {
    let expected_title_prefix = format!("# ADR-{number:04}:");
    let first_non_empty_line = content.lines().map(str::trim).find(|line| !line.is_empty());
    match first_non_empty_line {
        Some(line) if line.starts_with(&expected_title_prefix) => {}
        _ => {
            return Err(format!(
                "first non-empty line must start with `{expected_title_prefix}`"
            ));
        }
    }

    let status = parse_single_metadata_value(content, "Status")?;
    if !ALLOWED_ADR_STATUSES.contains(&status.as_str()) {
        return Err(format!(
            "status must be one of {:?}, found `{status}`",
            ALLOWED_ADR_STATUSES
        ));
    }

    let date = parse_single_metadata_value(content, "Date")?;
    if !is_valid_iso_date(&date) {
        return Err(format!(
            "date must be ISO-8601 calendar format `YYYY-MM-DD`, found `{date}`"
        ));
    }

    validate_required_sections(content)
}

fn assert_adr_content_contract(path: &Path, number: u32, content: &str) {
    validate_adr_content_contract(number, content).unwrap_or_else(|err| {
        panic!(
            "{} failed ADR content contract validation: {err}",
            path.display()
        )
    });
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
fn parse_single_metadata_value_requires_single_non_empty_line() {
    let content = "- Status: Accepted\n- Date: 2026-02-28\n";
    assert_eq!(
        parse_single_metadata_value(content, "Status"),
        Ok("Accepted".to_owned())
    );
    assert_eq!(
        parse_single_metadata_value(content, "Date"),
        Ok("2026-02-28".to_owned())
    );
    assert_eq!(
        parse_single_metadata_value(content, "Missing"),
        Err("missing required metadata line `- Missing:`".to_owned())
    );
    assert_eq!(
        parse_single_metadata_value("- Status:   \n", "Status"),
        Err("metadata line `- Status:` must not be empty".to_owned())
    );
    assert_eq!(
        parse_single_metadata_value("- Status: Accepted\n- Status: Proposed\n", "Status"),
        Err("metadata line `- Status:` must appear exactly once".to_owned())
    );
}

#[test]
fn is_valid_iso_date_enforces_calendar_boundaries() {
    for valid in ["2026-02-28", "2024-02-29", "2000-02-29"] {
        assert!(is_valid_iso_date(valid), "{valid} should be valid");
    }

    for invalid in [
        "",
        "2026-2-28",
        "2026-13-01",
        "2026-00-01",
        "2026-04-31",
        "1900-02-29",
        "2026-02-00",
        "2026/02/28",
    ] {
        assert!(!is_valid_iso_date(invalid), "{invalid} should be invalid");
    }
}

#[test]
fn validate_adr_content_contract_rejects_invalid_title_status_and_date() {
    let valid = "\
# ADR-0001: Example Decision

- Status: Accepted
- Date: 2026-02-28

## Context
Context text.

## Decision
Decision text.

## Consequences
Consequence text.
";
    assert_eq!(validate_adr_content_contract(1, valid), Ok(()));

    let invalid_title = valid.replacen("# ADR-0001: Example Decision", "Intro", 1);
    assert_eq!(
        validate_adr_content_contract(1, &invalid_title),
        Err("first non-empty line must start with `# ADR-0001:`".to_owned())
    );

    let invalid_status = valid.replacen("- Status: Accepted", "- Status: accepted", 1);
    assert_eq!(
        validate_adr_content_contract(1, &invalid_status),
        Err(
            "status must be one of [\"Proposed\", \"Accepted\", \"Superseded\", \"Rejected\", \"Deprecated\"], found `accepted`"
                .to_owned()
        )
    );

    let invalid_date = valid.replacen("- Date: 2026-02-28", "- Date: 2026-02-30", 1);
    assert_eq!(
        validate_adr_content_contract(1, &invalid_date),
        Err("date must be ISO-8601 calendar format `YYYY-MM-DD`, found `2026-02-30`".to_owned())
    );
}

#[test]
fn validate_required_sections_rejects_missing_duplicate_order_and_empty_bodies() {
    let valid = "\
## Context
Context text.

## Decision
Decision text.

## Consequences
Consequence text.
";
    assert_eq!(validate_required_sections(valid), Ok(()));

    let missing = valid.replacen("## Decision\nDecision text.\n\n", "", 1);
    assert_eq!(
        validate_required_sections(&missing),
        Err("missing required heading `## Decision`".to_owned())
    );

    let duplicate = valid.replacen(
        "## Consequences",
        "## Decision\nDecision extra.\n\n## Consequences",
        1,
    );
    assert_eq!(
        validate_required_sections(&duplicate),
        Err("required heading `## Decision` must appear exactly once".to_owned())
    );

    let out_of_order = "\
## Decision
Decision text.

## Context
Context text.

## Consequences
Consequence text.
";
    assert_eq!(
        validate_required_sections(out_of_order),
        Err("required heading `## Context` must appear before `## Decision`".to_owned())
    );

    let empty_section = valid.replacen("## Decision\nDecision text.\n\n", "## Decision\n\n", 1);
    assert_eq!(
        validate_required_sections(&empty_section),
        Err("section `## Decision` must contain at least one non-empty line".to_owned())
    );
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

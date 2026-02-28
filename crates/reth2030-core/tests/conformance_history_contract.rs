use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Map, Value};

const EXPECTED_METRIC: &str = "minimal-state-tests-pass-rate";
const EXPECTED_SUITE: &str = "minimal-state-tests";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()))
}

fn require_key<'a>(
    mapping: &'a Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<&'a Value, String> {
    mapping
        .get(key)
        .ok_or_else(|| format!("missing key `{key}` in {context}"))
}

fn as_object<'a>(value: &'a Value, context: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{context} must be a JSON object"))
}

fn as_array<'a>(value: &'a Value, context: &str) -> Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{context} must be a JSON array"))
}

fn as_str<'a>(value: &'a Value, context: &str) -> Result<&'a str, String> {
    value
        .as_str()
        .ok_or_else(|| format!("{context} must be a JSON string"))
}

fn as_u64(value: &Value, context: &str) -> Result<u64, String> {
    value
        .as_u64()
        .ok_or_else(|| format!("{context} must be a non-negative integer"))
}

fn as_f64(value: &Value, context: &str) -> Result<f64, String> {
    value
        .as_f64()
        .ok_or_else(|| format!("{context} must be a JSON number"))
}

fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn days_in_month(year: u32, month: u32) -> Option<u32> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if is_leap_year(year) => Some(29),
        2 => Some(28),
        _ => None,
    }
}

fn parse_iso_date(date: &str) -> Option<(u32, u32, u32)> {
    let bytes = date.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| (index == 4 || index == 7) || byte.is_ascii_digit())
    {
        return None;
    }

    let year = date[0..4].parse::<u32>().ok()?;
    let month = date[5..7].parse::<u32>().ok()?;
    let day = date[8..10].parse::<u32>().ok()?;
    let max_day = days_in_month(year, month)?;
    if day == 0 || day > max_day {
        return None;
    }

    Some((year, month, day))
}

fn validate_history(history: &Value) -> Result<(), String> {
    let root = as_object(history, "conformance history root")?;
    let metric = as_str(
        require_key(root, "metric", "conformance history root")?,
        "conformance history root.metric",
    )?;
    if metric != EXPECTED_METRIC {
        return Err(format!(
            "conformance history root.metric must be `{EXPECTED_METRIC}`, found `{metric}`"
        ));
    }

    let entries = as_array(
        require_key(root, "entries", "conformance history root")?,
        "conformance history root.entries",
    )?;
    if entries.is_empty() {
        return Err("conformance history must include at least one entry".to_string());
    }

    let mut seen_dates = HashSet::new();
    let mut previous_date: Option<String> = None;
    for (index, entry_value) in entries.iter().enumerate() {
        let entry_context = format!("conformance history root.entries[{index}]");
        let entry = as_object(entry_value, &entry_context)?;

        let date_context = format!("{entry_context}.recorded_on");
        let recorded_on = as_str(
            require_key(entry, "recorded_on", &entry_context)?,
            &date_context,
        )?;
        if parse_iso_date(recorded_on).is_none() {
            return Err(format!(
                "{date_context} must use a valid YYYY-MM-DD date, found `{recorded_on}`"
            ));
        }
        if !seen_dates.insert(recorded_on.to_string()) {
            return Err(format!(
                "{date_context} must be unique; duplicate date `{recorded_on}`"
            ));
        }
        if let Some(previous) = &previous_date {
            if recorded_on <= previous {
                return Err(format!(
                    "{date_context} must be strictly increasing; previous=`{previous}` current=`{recorded_on}`"
                ));
            }
        }
        previous_date = Some(recorded_on.to_string());

        let suite = as_str(
            require_key(entry, "suite", &entry_context)?,
            &format!("{entry_context}.suite"),
        )?;
        if suite != EXPECTED_SUITE {
            return Err(format!(
                "{entry_context}.suite must be `{EXPECTED_SUITE}`, found `{suite}`"
            ));
        }

        let total = as_u64(
            require_key(entry, "total", &entry_context)?,
            &format!("{entry_context}.total"),
        )?;
        let passed = as_u64(
            require_key(entry, "passed", &entry_context)?,
            &format!("{entry_context}.passed"),
        )?;
        let failed = as_u64(
            require_key(entry, "failed", &entry_context)?,
            &format!("{entry_context}.failed"),
        )?;
        let Some(computed_total) = passed.checked_add(failed) else {
            return Err(format!(
                "{entry_context} has overflow while computing passed + failed"
            ));
        };
        if total != computed_total {
            return Err(format!(
                "{entry_context} must satisfy total == passed + failed"
            ));
        }

        let pass_rate = as_f64(
            require_key(entry, "pass_rate", &entry_context)?,
            &format!("{entry_context}.pass_rate"),
        )?;
        if !pass_rate.is_finite() {
            return Err(format!("{entry_context}.pass_rate must be finite"));
        }
        if !(0.0..=1.0).contains(&pass_rate) {
            return Err(format!(
                "{entry_context}.pass_rate must be in the inclusive range [0.0, 1.0]"
            ));
        }
        let expected_pass_rate = if total == 0 {
            0.0
        } else {
            (passed as f64) / (total as f64)
        };
        if (pass_rate - expected_pass_rate).abs() > 1e-12 {
            return Err(format!(
                "{entry_context}.pass_rate must equal passed / total"
            ));
        }
    }

    Ok(())
}

fn history_fixture(entries: Value) -> Value {
    json!({
        "metric": EXPECTED_METRIC,
        "description": "fixture",
        "entries": entries
    })
}

#[test]
fn parse_iso_date_accepts_and_rejects_calendar_edge_cases() {
    assert_eq!(parse_iso_date("2024-02-29"), Some((2024, 2, 29)));
    assert_eq!(parse_iso_date("2023-02-29"), None);
    assert_eq!(parse_iso_date("2026-13-01"), None);
    assert_eq!(parse_iso_date("2026-04-31"), None);
    assert_eq!(parse_iso_date("2026-00-01"), None);
    assert_eq!(parse_iso_date("2026-01-00"), None);
    assert_eq!(parse_iso_date("2026/01/01"), None);
}

#[test]
fn validate_history_accepts_valid_entries_with_zero_total_case() {
    let history = history_fixture(json!([
        {
            "recorded_on": "2026-02-27",
            "suite": EXPECTED_SUITE,
            "total": 0,
            "passed": 0,
            "failed": 0,
            "pass_rate": 0.0
        },
        {
            "recorded_on": "2026-02-28",
            "suite": EXPECTED_SUITE,
            "total": 4,
            "passed": 4,
            "failed": 0,
            "pass_rate": 1.0
        }
    ]));

    validate_history(&history).expect("history should validate");
}

#[test]
fn validate_history_rejects_empty_entries() {
    let history = history_fixture(json!([]));
    let err = validate_history(&history).expect_err("empty history must fail");
    assert!(err.contains("at least one entry"));
}

#[test]
fn validate_history_rejects_duplicate_or_non_monotonic_dates() {
    let duplicate_dates = history_fixture(json!([
        {
            "recorded_on": "2026-02-28",
            "suite": EXPECTED_SUITE,
            "total": 1,
            "passed": 1,
            "failed": 0,
            "pass_rate": 1.0
        },
        {
            "recorded_on": "2026-02-28",
            "suite": EXPECTED_SUITE,
            "total": 1,
            "passed": 1,
            "failed": 0,
            "pass_rate": 1.0
        }
    ]));
    let err = validate_history(&duplicate_dates).expect_err("duplicate dates must fail");
    assert!(err.contains("duplicate date"));

    let non_monotonic_dates = history_fixture(json!([
        {
            "recorded_on": "2026-02-28",
            "suite": EXPECTED_SUITE,
            "total": 1,
            "passed": 1,
            "failed": 0,
            "pass_rate": 1.0
        },
        {
            "recorded_on": "2026-02-27",
            "suite": EXPECTED_SUITE,
            "total": 1,
            "passed": 1,
            "failed": 0,
            "pass_rate": 1.0
        }
    ]));
    let err = validate_history(&non_monotonic_dates).expect_err("date ordering must fail");
    assert!(err.contains("strictly increasing"));
}

#[test]
fn validate_history_rejects_mismatched_counts_and_pass_rate_drift() {
    let bad_counts = history_fixture(json!([
        {
            "recorded_on": "2026-02-28",
            "suite": EXPECTED_SUITE,
            "total": 3,
            "passed": 2,
            "failed": 0,
            "pass_rate": 0.66
        }
    ]));
    let err = validate_history(&bad_counts).expect_err("count mismatch must fail");
    assert!(err.contains("total == passed + failed"));

    let bad_pass_rate = history_fixture(json!([
        {
            "recorded_on": "2026-02-28",
            "suite": EXPECTED_SUITE,
            "total": 4,
            "passed": 3,
            "failed": 1,
            "pass_rate": 1.0
        }
    ]));
    let err = validate_history(&bad_pass_rate).expect_err("pass-rate drift must fail");
    assert!(err.contains("pass_rate must equal passed / total"));

    let out_of_range = history_fixture(json!([
        {
            "recorded_on": "2026-02-28",
            "suite": EXPECTED_SUITE,
            "total": 4,
            "passed": 4,
            "failed": 0,
            "pass_rate": 1.5
        }
    ]));
    let err = validate_history(&out_of_range).expect_err("out-of-range pass-rate must fail");
    assert!(err.contains("inclusive range [0.0, 1.0]"));
}

#[test]
fn docs_reference_conformance_history_tracking() {
    let docs = read_repo_file("docs/conformance.md");
    assert!(
        docs.contains("vectors/baseline/conformance-history.json"),
        "docs/conformance.md must reference the conformance history artifact"
    );
    assert!(
        docs.contains(EXPECTED_METRIC),
        "docs/conformance.md must name the tracked conformance metric"
    );
    assert!(
        docs.contains("recorded_on"),
        "docs/conformance.md must describe the history entry date field"
    );
}

#[test]
fn checked_in_conformance_history_is_valid_and_latest_matches_scorecard() {
    let history_contents = read_repo_file("vectors/baseline/conformance-history.json");
    let history: Value =
        serde_json::from_str(&history_contents).expect("conformance history must be valid JSON");
    validate_history(&history).expect("conformance history invariants must hold");

    let history_root = as_object(&history, "conformance history root").expect("history object");
    let entries = as_array(
        require_key(history_root, "entries", "conformance history root")
            .expect("history.entries key"),
        "conformance history root.entries",
    )
    .expect("history entries array");
    let latest_entry = as_object(
        entries.last().expect("validated non-empty entries"),
        "latest entry",
    )
    .expect("latest entry object");

    let scorecard_contents = read_repo_file("vectors/baseline/scorecard.json");
    let scorecard: Value =
        serde_json::from_str(&scorecard_contents).expect("scorecard must be valid JSON");
    let scorecard_root = as_object(&scorecard, "scorecard root").expect("scorecard object");

    assert_eq!(
        as_str(
            require_key(latest_entry, "suite", "latest history entry").expect("latest suite key"),
            "latest history entry.suite",
        )
        .expect("latest suite string"),
        as_str(
            require_key(scorecard_root, "suite", "scorecard root").expect("scorecard suite key"),
            "scorecard.suite",
        )
        .expect("scorecard suite string")
    );
    assert_eq!(
        as_u64(
            require_key(latest_entry, "total", "latest history entry").expect("latest total key"),
            "latest history entry.total",
        )
        .expect("latest total"),
        as_u64(
            require_key(scorecard_root, "total", "scorecard root").expect("scorecard total key"),
            "scorecard.total",
        )
        .expect("scorecard total")
    );
    assert_eq!(
        as_u64(
            require_key(latest_entry, "passed", "latest history entry").expect("latest passed key"),
            "latest history entry.passed",
        )
        .expect("latest passed"),
        as_u64(
            require_key(scorecard_root, "passed", "scorecard root").expect("scorecard passed key"),
            "scorecard.passed",
        )
        .expect("scorecard passed")
    );
    assert_eq!(
        as_u64(
            require_key(latest_entry, "failed", "latest history entry").expect("latest failed key"),
            "latest history entry.failed",
        )
        .expect("latest failed"),
        as_u64(
            require_key(scorecard_root, "failed", "scorecard root").expect("scorecard failed key"),
            "scorecard.failed",
        )
        .expect("scorecard failed")
    );

    let latest_pass_rate = as_f64(
        require_key(latest_entry, "pass_rate", "latest history entry")
            .expect("latest pass_rate key"),
        "latest history entry.pass_rate",
    )
    .expect("latest pass_rate");
    let scorecard_pass_rate = as_f64(
        require_key(scorecard_root, "pass_rate", "scorecard root")
            .expect("scorecard pass_rate key"),
        "scorecard.pass_rate",
    )
    .expect("scorecard pass_rate");
    assert!(
        (latest_pass_rate - scorecard_pass_rate).abs() <= 1e-12,
        "latest history entry pass_rate must match checked-in scorecard"
    );
}

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Map, Value};

const TODO_ACCEPTANCE_CRITERION_LINE: &str =
    "- [x] A documented conformance metric is tracked over time.";
const EXPECTED_METRIC: &str = "minimal-state-tests-pass-rate";
const EXPECTED_SUITE: &str = "minimal-state-tests";
const EXPECTED_DESCRIPTION_FIXTURE_PATH: &str = "vectors/ethereum-state-tests/minimal";
const ALLOWED_HISTORY_ROOT_KEYS: [&str; 3] = ["metric", "description", "entries"];
const ALLOWED_HISTORY_ENTRY_KEYS: [&str; 6] = [
    "recorded_on",
    "suite",
    "total",
    "passed",
    "failed",
    "pass_rate",
];
const EXPECTED_BOOTSTRAP_RECORDED_ON: &str = "2026-02-27";
const EXPECTED_BOOTSTRAP_TOTAL: u64 = 4;
const EXPECTED_BOOTSTRAP_PASSED: u64 = 3;
const EXPECTED_BOOTSTRAP_FAILED: u64 = 1;
const EXPECTED_BOOTSTRAP_PASS_RATE: f64 = 0.75;

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

fn reject_unknown_keys(
    mapping: &Map<String, Value>,
    allowed_keys: &[&str],
    context: &str,
) -> Result<(), String> {
    for key in mapping.keys() {
        if !allowed_keys.contains(&key.as_str()) {
            return Err(format!("{context} contains unexpected key `{key}`"));
        }
    }
    Ok(())
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

fn validate_time_series_history(history: &Value) -> Result<(), String> {
    let root = as_object(history, "conformance history root")?;
    reject_unknown_keys(root, &ALLOWED_HISTORY_ROOT_KEYS, "conformance history root")?;
    let metric = as_str(
        require_key(root, "metric", "conformance history root")?,
        "conformance history root.metric",
    )?;
    if metric != EXPECTED_METRIC {
        return Err(format!(
            "conformance history root.metric must be `{EXPECTED_METRIC}`, found `{metric}`"
        ));
    }

    let description = as_str(
        require_key(root, "description", "conformance history root")?,
        "conformance history root.description",
    )?;
    if description.trim().is_empty() {
        return Err("conformance history root.description must be non-empty".to_string());
    }
    if !description.contains(EXPECTED_DESCRIPTION_FIXTURE_PATH) {
        return Err(format!(
            "conformance history root.description must reference `{EXPECTED_DESCRIPTION_FIXTURE_PATH}`"
        ));
    }

    let entries = as_array(
        require_key(root, "entries", "conformance history root")?,
        "conformance history root.entries",
    )?;
    if entries.len() < 2 {
        return Err(format!(
            "conformance history must contain at least two entries to be tracked over time; found {}",
            entries.len()
        ));
    }

    let mut seen_dates = HashSet::new();
    let mut previous_date: Option<String> = None;
    for (index, entry_value) in entries.iter().enumerate() {
        let entry_context = format!("conformance history root.entries[{index}]");
        let entry = as_object(entry_value, &entry_context)?;
        reject_unknown_keys(entry, &ALLOWED_HISTORY_ENTRY_KEYS, &entry_context)?;

        let recorded_on = as_str(
            require_key(entry, "recorded_on", &entry_context)?,
            &format!("{entry_context}.recorded_on"),
        )?;
        if parse_iso_date(recorded_on).is_none() {
            return Err(format!(
                "{entry_context}.recorded_on must be a valid YYYY-MM-DD date"
            ));
        }
        if !seen_dates.insert(recorded_on.to_owned()) {
            return Err(format!(
                "{entry_context}.recorded_on must be unique; duplicate date `{recorded_on}`"
            ));
        }
        if let Some(previous) = &previous_date {
            if recorded_on <= previous {
                return Err(format!(
                    "{entry_context}.recorded_on must be strictly increasing; previous=`{previous}` current=`{recorded_on}`"
                ));
            }
        }
        previous_date = Some(recorded_on.to_owned());

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
                "{entry_context} overflows while computing passed + failed"
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

fn validate_latest_entry_matches_scorecard(
    history: &Value,
    scorecard: &Value,
) -> Result<(), String> {
    let history_root = as_object(history, "conformance history root")?;
    let entries = as_array(
        require_key(history_root, "entries", "conformance history root")?,
        "conformance history root.entries",
    )?;
    let latest_entry = as_object(
        entries
            .last()
            .ok_or_else(|| "history entries must not be empty".to_string())?,
        "latest history entry",
    )?;

    let scorecard_root = as_object(scorecard, "scorecard root")?;

    for field in ["suite", "total", "passed", "failed"] {
        let latest = require_key(latest_entry, field, "latest history entry")?;
        let scorecard = require_key(scorecard_root, field, "scorecard root")?;
        if latest != scorecard {
            return Err(format!(
                "latest history entry `{field}` must match scorecard (`latest={latest}`, `scorecard={scorecard}`)"
            ));
        }
    }

    let latest_pass_rate = as_f64(
        require_key(latest_entry, "pass_rate", "latest history entry")?,
        "latest history entry.pass_rate",
    )?;
    let scorecard_pass_rate = as_f64(
        require_key(scorecard_root, "pass_rate", "scorecard root")?,
        "scorecard.pass_rate",
    )?;
    if (latest_pass_rate - scorecard_pass_rate).abs() > 1e-12 {
        return Err("latest history entry pass_rate must match scorecard".to_string());
    }

    Ok(())
}

fn validate_bootstrap_entry(history: &Value) -> Result<(), String> {
    let history_root = as_object(history, "conformance history root")?;
    let entries = as_array(
        require_key(history_root, "entries", "conformance history root")?,
        "conformance history root.entries",
    )?;
    let first_entry = as_object(
        entries
            .first()
            .ok_or_else(|| "history entries must not be empty".to_string())?,
        "bootstrap history entry",
    )?;

    let recorded_on = as_str(
        require_key(first_entry, "recorded_on", "bootstrap history entry")?,
        "bootstrap history entry.recorded_on",
    )?;
    if recorded_on != EXPECTED_BOOTSTRAP_RECORDED_ON {
        return Err(format!(
            "bootstrap history entry date must remain `{EXPECTED_BOOTSTRAP_RECORDED_ON}`, found `{recorded_on}`"
        ));
    }

    let suite = as_str(
        require_key(first_entry, "suite", "bootstrap history entry")?,
        "bootstrap history entry.suite",
    )?;
    if suite != EXPECTED_SUITE {
        return Err(format!(
            "bootstrap history entry suite must remain `{EXPECTED_SUITE}`, found `{suite}`"
        ));
    }

    let total = as_u64(
        require_key(first_entry, "total", "bootstrap history entry")?,
        "bootstrap history entry.total",
    )?;
    if total != EXPECTED_BOOTSTRAP_TOTAL {
        return Err(format!(
            "bootstrap history entry total must remain `{EXPECTED_BOOTSTRAP_TOTAL}`, found `{total}`"
        ));
    }

    let passed = as_u64(
        require_key(first_entry, "passed", "bootstrap history entry")?,
        "bootstrap history entry.passed",
    )?;
    if passed != EXPECTED_BOOTSTRAP_PASSED {
        return Err(format!(
            "bootstrap history entry passed must remain `{EXPECTED_BOOTSTRAP_PASSED}`, found `{passed}`"
        ));
    }

    let failed = as_u64(
        require_key(first_entry, "failed", "bootstrap history entry")?,
        "bootstrap history entry.failed",
    )?;
    if failed != EXPECTED_BOOTSTRAP_FAILED {
        return Err(format!(
            "bootstrap history entry failed must remain `{EXPECTED_BOOTSTRAP_FAILED}`, found `{failed}`"
        ));
    }

    let pass_rate = as_f64(
        require_key(first_entry, "pass_rate", "bootstrap history entry")?,
        "bootstrap history entry.pass_rate",
    )?;
    if (pass_rate - EXPECTED_BOOTSTRAP_PASS_RATE).abs() > 1e-12 {
        return Err(format!(
            "bootstrap history entry pass_rate must remain `{EXPECTED_BOOTSTRAP_PASS_RATE}`, found `{pass_rate}`"
        ));
    }

    Ok(())
}

fn history_fixture(entries: Value) -> Value {
    json!({
        "metric": EXPECTED_METRIC,
        "description": format!("Historical scorecard trend for {EXPECTED_DESCRIPTION_FIXTURE_PATH}."),
        "entries": entries
    })
}

#[test]
fn todo_marks_conformance_metric_acceptance_criterion_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines()
            .any(|line| line.trim() == TODO_ACCEPTANCE_CRITERION_LINE),
        "TODO.md must keep this acceptance criterion checked: {TODO_ACCEPTANCE_CRITERION_LINE}"
    );
}

#[test]
fn validate_time_series_history_accepts_valid_two_entry_timeline() {
    let history = history_fixture(json!([
        {
            "recorded_on": "2026-02-27",
            "suite": EXPECTED_SUITE,
            "total": 4,
            "passed": 3,
            "failed": 1,
            "pass_rate": 0.75
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

    validate_time_series_history(&history).expect("valid multi-entry timeline should pass");
}

#[test]
fn validate_time_series_history_rejects_single_entry_or_non_monotonic_dates() {
    let single_entry_history = history_fixture(json!([{
        "recorded_on": "2026-02-28",
        "suite": EXPECTED_SUITE,
        "total": 4,
        "passed": 4,
        "failed": 0,
        "pass_rate": 1.0
    }]));
    let err = validate_time_series_history(&single_entry_history)
        .expect_err("single-entry timeline must fail");
    assert!(err.contains("at least two entries"));

    let duplicate_dates = history_fixture(json!([
        {
            "recorded_on": "2026-02-28",
            "suite": EXPECTED_SUITE,
            "total": 4,
            "passed": 4,
            "failed": 0,
            "pass_rate": 1.0
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
    let err =
        validate_time_series_history(&duplicate_dates).expect_err("duplicate dates must fail");
    assert!(err.contains("duplicate"));

    let non_monotonic = history_fixture(json!([
        {
            "recorded_on": "2026-02-28",
            "suite": EXPECTED_SUITE,
            "total": 4,
            "passed": 4,
            "failed": 0,
            "pass_rate": 1.0
        },
        {
            "recorded_on": "2026-02-27",
            "suite": EXPECTED_SUITE,
            "total": 4,
            "passed": 4,
            "failed": 0,
            "pass_rate": 1.0
        }
    ]));
    let err =
        validate_time_series_history(&non_monotonic).expect_err("non-monotonic dates must fail");
    assert!(err.contains("strictly increasing"));
}

#[test]
fn validate_time_series_history_rejects_missing_or_undocumented_description() {
    let missing_description = json!({
        "metric": EXPECTED_METRIC,
        "entries": [
            {
                "recorded_on": "2026-02-27",
                "suite": EXPECTED_SUITE,
                "total": 4,
                "passed": 3,
                "failed": 1,
                "pass_rate": 0.75
            },
            {
                "recorded_on": "2026-02-28",
                "suite": EXPECTED_SUITE,
                "total": 4,
                "passed": 4,
                "failed": 0,
                "pass_rate": 1.0
            }
        ]
    });
    let err = validate_time_series_history(&missing_description)
        .expect_err("missing description must fail validation");
    assert!(err.contains("missing key `description`"));

    let empty_description = json!({
        "metric": EXPECTED_METRIC,
        "description": "   ",
        "entries": [
            {
                "recorded_on": "2026-02-27",
                "suite": EXPECTED_SUITE,
                "total": 4,
                "passed": 3,
                "failed": 1,
                "pass_rate": 0.75
            },
            {
                "recorded_on": "2026-02-28",
                "suite": EXPECTED_SUITE,
                "total": 4,
                "passed": 4,
                "failed": 0,
                "pass_rate": 1.0
            }
        ]
    });
    let err = validate_time_series_history(&empty_description)
        .expect_err("empty description must fail validation");
    assert!(err.contains("must be non-empty"));

    let undocumented_description = json!({
        "metric": EXPECTED_METRIC,
        "description": "Historical scorecard trend for a local fixture suite.",
        "entries": [
            {
                "recorded_on": "2026-02-27",
                "suite": EXPECTED_SUITE,
                "total": 4,
                "passed": 3,
                "failed": 1,
                "pass_rate": 0.75
            },
            {
                "recorded_on": "2026-02-28",
                "suite": EXPECTED_SUITE,
                "total": 4,
                "passed": 4,
                "failed": 0,
                "pass_rate": 1.0
            }
        ]
    });
    let err = validate_time_series_history(&undocumented_description)
        .expect_err("description without fixture reference must fail validation");
    assert!(err.contains("must reference"));
}

#[test]
fn validate_time_series_history_rejects_unknown_root_or_entry_fields() {
    let unknown_root_field = json!({
        "metric": EXPECTED_METRIC,
        "description": format!("Historical scorecard trend for {EXPECTED_DESCRIPTION_FIXTURE_PATH}."),
        "entries": [
            {
                "recorded_on": "2026-02-27",
                "suite": EXPECTED_SUITE,
                "total": 4,
                "passed": 3,
                "failed": 1,
                "pass_rate": 0.75
            },
            {
                "recorded_on": "2026-02-28",
                "suite": EXPECTED_SUITE,
                "total": 4,
                "passed": 4,
                "failed": 0,
                "pass_rate": 1.0
            }
        ],
        "notes": "extra metadata should fail closed"
    });
    let err = validate_time_series_history(&unknown_root_field)
        .expect_err("unexpected root fields must fail validation");
    assert!(err.contains("unexpected key `notes`"));

    let unknown_entry_field = history_fixture(json!([
        {
            "recorded_on": "2026-02-27",
            "suite": EXPECTED_SUITE,
            "total": 4,
            "passed": 3,
            "failed": 1,
            "pass_rate": 0.75,
            "note": "extra metadata should fail closed"
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
    let err = validate_time_series_history(&unknown_entry_field)
        .expect_err("unexpected entry fields must fail validation");
    assert!(err.contains("unexpected key `note`"));
}

#[test]
fn validate_time_series_history_rejects_invalid_iso_dates() {
    for invalid_date in ["2026-02-29", "2026-13-01", "2026/02/28", "2026-2-28"] {
        let history = history_fixture(json!([
            {
                "recorded_on": invalid_date,
                "suite": EXPECTED_SUITE,
                "total": 4,
                "passed": 3,
                "failed": 1,
                "pass_rate": 0.75
            },
            {
                "recorded_on": "2026-03-01",
                "suite": EXPECTED_SUITE,
                "total": 4,
                "passed": 4,
                "failed": 0,
                "pass_rate": 1.0
            }
        ]));

        let err = validate_time_series_history(&history)
            .expect_err("invalid recorded_on date must fail validation");
        assert!(
            err.contains("valid YYYY-MM-DD"),
            "expected invalid-date error for `{invalid_date}`, got `{err}`"
        );
    }
}

#[test]
fn validate_time_series_history_rejects_pass_fail_overflow() {
    let history = history_fixture(json!([
        {
            "recorded_on": "2026-02-27",
            "suite": EXPECTED_SUITE,
            "total": 4,
            "passed": 3,
            "failed": 1,
            "pass_rate": 0.75
        },
        {
            "recorded_on": "2026-02-28",
            "suite": EXPECTED_SUITE,
            "total": u64::MAX,
            "passed": u64::MAX,
            "failed": 1,
            "pass_rate": 1.0
        }
    ]));

    let err = validate_time_series_history(&history)
        .expect_err("overflowed passed + failed must fail validation");
    assert!(err.contains("overflows while computing passed + failed"));
}

#[test]
fn validate_latest_entry_matches_scorecard_rejects_value_drift() {
    let history = history_fixture(json!([
        {
            "recorded_on": "2026-02-27",
            "suite": EXPECTED_SUITE,
            "total": 4,
            "passed": 4,
            "failed": 0,
            "pass_rate": 1.0
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
    let scorecard = json!({
        "suite": EXPECTED_SUITE,
        "total": 4,
        "passed": 3,
        "failed": 1,
        "pass_rate": 0.75
    });

    let err = validate_latest_entry_matches_scorecard(&history, &scorecard)
        .expect_err("latest-entry scorecard mismatch must fail");
    assert!(err.contains("must match scorecard"));
}

#[test]
fn validate_latest_entry_matches_scorecard_rejects_pass_rate_drift() {
    let history = history_fixture(json!([
        {
            "recorded_on": "2026-02-27",
            "suite": EXPECTED_SUITE,
            "total": 4,
            "passed": 3,
            "failed": 1,
            "pass_rate": 0.75
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
    let scorecard = json!({
        "suite": EXPECTED_SUITE,
        "total": 4,
        "passed": 4,
        "failed": 0,
        "pass_rate": 0.99
    });

    let err = validate_latest_entry_matches_scorecard(&history, &scorecard)
        .expect_err("latest-entry pass-rate mismatch must fail");
    assert!(err.contains("pass_rate must match scorecard"));
}

#[test]
fn validate_bootstrap_entry_rejects_rewritten_baseline_history() {
    let rewritten_history = history_fixture(json!([
        {
            "recorded_on": EXPECTED_BOOTSTRAP_RECORDED_ON,
            "suite": EXPECTED_SUITE,
            "total": 4,
            "passed": 4,
            "failed": 0,
            "pass_rate": 1.0
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

    let err =
        validate_bootstrap_entry(&rewritten_history).expect_err("rewriting first entry must fail");
    assert!(err.contains("bootstrap history entry"));
}

#[test]
fn checked_in_metric_timeline_has_multiple_entries_and_docs_tracking_contract() {
    let history_contents = read_repo_file("vectors/baseline/conformance-history.json");
    let history: Value =
        serde_json::from_str(&history_contents).expect("history artifact must remain valid JSON");
    validate_time_series_history(&history)
        .expect("checked-in conformance history must remain a valid time-series timeline");
    validate_bootstrap_entry(&history)
        .expect("checked-in conformance history must preserve bootstrap entry continuity");

    let scorecard_contents = read_repo_file("vectors/baseline/scorecard.json");
    let scorecard: Value = serde_json::from_str(&scorecard_contents)
        .expect("scorecard artifact must remain valid JSON");
    validate_latest_entry_matches_scorecard(&history, &scorecard)
        .expect("latest history entry must stay aligned with checked-in scorecard");

    let docs = read_repo_file("docs/conformance.md");
    assert!(
        docs.contains("vectors/baseline/conformance-history.json"),
        "docs/conformance.md must reference the conformance history timeline artifact"
    );
    assert!(
        docs.contains(EXPECTED_METRIC),
        "docs/conformance.md must explicitly name the tracked conformance metric"
    );
    assert!(
        docs.contains("append a new entry"),
        "docs/conformance.md must preserve append-only guidance for timeline updates"
    );
    assert!(
        docs.contains("YYYY-MM-DD"),
        "docs/conformance.md must preserve the recorded_on date format guidance"
    );
    assert!(
        docs.contains("strictly increasing"),
        "docs/conformance.md must preserve strict chronology guidance for timeline updates"
    );
    assert!(
        docs.contains("Do not edit or remove prior entries"),
        "docs/conformance.md must explicitly preserve historical entries"
    );
}

use clap::Parser;
use reth2030_core::{Account, InMemoryState, StateStore};
use reth2030_types::{LegacyTx, Transaction};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Parser)]
#[command(name = "reth2030-vectors")]
#[command(about = "Run vector fixtures and produce conformance reports")]
struct Args {
    #[arg(long, default_value = "vectors/ethereum-state-tests/minimal")]
    fixtures_dir: PathBuf,

    #[arg(long, default_value = "artifacts/vectors")]
    out_dir: PathBuf,

    #[arg(long, default_value = "vectors/baseline/scorecard.json")]
    baseline_scorecard: PathBuf,

    #[arg(long, default_value = "vectors/baseline/snapshot.json")]
    baseline_snapshot: PathBuf,

    #[arg(long, default_value_t = false)]
    update_baseline: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    name: String,
    initial_accounts: Vec<FixtureBalance>,
    transactions: Vec<FixtureTx>,
    expected: FixtureExpected,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureBalance {
    address: String,
    balance: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureTx {
    from: String,
    to: Option<String>,
    nonce: u64,
    value: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureExpected {
    success: bool,
    balances: Vec<FixtureBalance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FixtureRun {
    name: String,
    passed: bool,
    expected_success: bool,
    actual_success: bool,
    mismatches: Vec<String>,
    actual_balances: Vec<BalanceSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct BalanceSnapshot {
    address: String,
    balance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Scorecard {
    suite: String,
    total: usize,
    passed: usize,
    failed: usize,
    pass_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct SnapshotReport {
    suite: String,
    fixtures: Vec<FixtureRun>,
}

const SUITE_NAME: &str = "minimal-state-tests";

fn main() {
    if let Err(err) = run() {
        eprintln!("reth2030-vectors: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse();
    let fixtures = load_fixtures(&args.fixtures_dir)?;
    if fixtures.is_empty() {
        return Err(format!(
            "no fixtures found in {}",
            args.fixtures_dir.display()
        ));
    }
    let (scorecard, snapshot) = generate_reports(&fixtures)?;
    let total = scorecard.total;
    let passed = scorecard.passed;
    let failed = scorecard.failed;
    let pass_rate = scorecard.pass_rate;

    let scorecard_json = serde_json::to_string_pretty(&scorecard).map_err(|err| err.to_string())?;
    let snapshot_json = serde_json::to_string_pretty(&snapshot).map_err(|err| err.to_string())?;

    fs::create_dir_all(&args.out_dir).map_err(|err| err.to_string())?;
    let out_scorecard = args.out_dir.join("scorecard.json");
    let out_snapshot = args.out_dir.join("snapshot.json");

    fs::write(&out_scorecard, &scorecard_json).map_err(|err| err.to_string())?;
    fs::write(&out_snapshot, &snapshot_json).map_err(|err| err.to_string())?;

    if args.update_baseline {
        if let Some(parent) = args.baseline_scorecard.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        if let Some(parent) = args.baseline_snapshot.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }

        fs::write(&args.baseline_scorecard, &scorecard_json).map_err(|err| err.to_string())?;
        fs::write(&args.baseline_snapshot, &snapshot_json).map_err(|err| err.to_string())?;
    } else {
        compare_with_baseline("scorecard", &args.baseline_scorecard, &scorecard_json)?;
        compare_with_baseline("snapshot", &args.baseline_snapshot, &snapshot_json)?;
    }

    println!("fixtures: {}", total);
    println!("passed: {}", passed);
    println!("failed: {}", failed);
    println!("pass_rate: {:.2}", pass_rate);
    println!("scorecard: {}", out_scorecard.display());
    println!("snapshot: {}", out_snapshot.display());

    if failed > 0 {
        return Err(format!(
            "{} fixture(s) failed semantic checks; see snapshot output",
            failed
        ));
    }

    Ok(())
}

fn generate_reports(fixtures: &[Fixture]) -> Result<(Scorecard, SnapshotReport), String> {
    let mut runs = Vec::with_capacity(fixtures.len());
    for fixture in fixtures {
        runs.push(execute_fixture(fixture)?);
    }
    runs.sort_by(|a, b| a.name.cmp(&b.name));

    let passed = runs.iter().filter(|run| run.passed).count();
    let total = runs.len();
    let failed = total.saturating_sub(passed);
    let pass_rate = if total == 0 {
        0.0
    } else {
        (passed as f64) / (total as f64)
    };

    let scorecard = Scorecard {
        suite: SUITE_NAME.to_string(),
        total,
        passed,
        failed,
        pass_rate,
    };
    let snapshot = SnapshotReport {
        suite: SUITE_NAME.to_string(),
        fixtures: runs,
    };

    Ok((scorecard, snapshot))
}

fn load_fixtures(fixtures_dir: &Path) -> Result<Vec<Fixture>, String> {
    let paths = collect_fixture_paths(fixtures_dir)?;

    let mut fixtures = Vec::with_capacity(paths.len());
    let mut seen_names = BTreeMap::new();
    for path in paths {
        let contents = fs::read_to_string(&path).map_err(|err| err.to_string())?;
        let fixture: Fixture = serde_json::from_str(&contents)
            .map_err(|err| format!("failed to decode {}: {}", path.display(), err))?;

        if let Some(previous_path) = seen_names.insert(fixture.name.clone(), path.clone()) {
            return Err(format!(
                "duplicate fixture name '{}' in {} and {}",
                fixture.name,
                previous_path.display(),
                path.display()
            ));
        }

        fixtures.push(fixture);
    }

    Ok(fixtures)
}

fn collect_fixture_paths(fixtures_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    collect_fixture_paths_recursive(fixtures_dir, &mut paths)?;
    Ok(paths)
}

fn collect_fixture_paths_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|err| {
        format!(
            "failed to read fixtures directory {}: {}",
            dir.display(),
            err
        )
    })?;
    let mut paths = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|err| err.to_string())?;
        paths.push(entry.path());
    }

    paths.sort();

    for path in paths {
        if path.is_dir() {
            collect_fixture_paths_recursive(&path, out)?;
            continue;
        }

        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            out.push(path);
        }
    }

    Ok(())
}

fn execute_fixture(fixture: &Fixture) -> Result<FixtureRun, String> {
    let mut state = InMemoryState::new();

    for account in &fixture.initial_accounts {
        let address = parse_address(&account.address)?;
        let balance = parse_u128(&account.balance)?;
        state.upsert_account(
            address,
            Account {
                balance,
                ..Account::default()
            },
        );
    }

    let mut txs = Vec::with_capacity(fixture.transactions.len());
    for tx in &fixture.transactions {
        let from = parse_address(&tx.from)?;
        let to = tx.to.as_deref().map(parse_address).transpose()?;
        let value = parse_u128(&tx.value)?;

        txs.push(Transaction::Legacy(LegacyTx {
            nonce: tx.nonce,
            from,
            to,
            gas_limit: 21_000,
            gas_price: 1,
            value,
            data: Vec::new(),
        }));
    }

    let actual_success = state.apply_transactions(&txs).is_ok();
    let mut mismatches = Vec::new();

    if actual_success != fixture.expected.success {
        mismatches.push(format!(
            "success mismatch: expected={}, actual={}",
            fixture.expected.success, actual_success
        ));
    }

    let mut expected_balances = BTreeMap::new();
    for expected_balance in &fixture.expected.balances {
        let address = parse_address(&expected_balance.address)?;
        let expected_value = parse_u128(&expected_balance.balance)?;

        if expected_balances.insert(address, expected_value).is_some() {
            return Err(format!(
                "fixture '{}' has duplicate expected balance entry for {}",
                fixture.name, expected_balance.address
            ));
        }
    }

    let snapshot = state.snapshot();

    for (address, expected_value) in &expected_balances {
        let actual_value = snapshot
            .get(address)
            .map(|account| account.balance)
            .unwrap_or(0);
        if actual_value != *expected_value {
            mismatches.push(format!(
                "balance mismatch for {}: expected={}, actual={}",
                format_address(address),
                expected_value,
                actual_value
            ));
        }
    }

    for (address, account) in &snapshot {
        if !expected_balances.contains_key(address) {
            mismatches.push(format!(
                "unexpected account in post-state {} with balance={}",
                format_address(address),
                account.balance
            ));
        }
    }

    let address_union: BTreeSet<[u8; 20]> = snapshot
        .keys()
        .copied()
        .chain(expected_balances.keys().copied())
        .collect();
    let mut actual_balances = Vec::with_capacity(address_union.len());
    for address in address_union {
        let balance = snapshot
            .get(&address)
            .map(|account| account.balance)
            .unwrap_or(0);
        actual_balances.push(BalanceSnapshot {
            address: format_address(&address),
            balance: balance.to_string(),
        });
    }
    actual_balances.sort_by(|a, b| a.address.cmp(&b.address));

    Ok(FixtureRun {
        name: fixture.name.clone(),
        passed: mismatches.is_empty(),
        expected_success: fixture.expected.success,
        actual_success,
        mismatches,
        actual_balances,
    })
}

fn parse_u128(value: &str) -> Result<u128, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("invalid numeric value: {}", value));
    }

    let (radix, digits) = if let Some(hex_digits) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        (16, hex_digits)
    } else {
        (10, trimmed)
    };

    if digits.is_empty() {
        return Err(format!("invalid numeric value: {}", value));
    }

    u128::from_str_radix(digits, radix).map_err(|_| format!("invalid numeric value: {}", value))
}

fn parse_address(input: &str) -> Result<[u8; 20], String> {
    let hex = input.strip_prefix("0x").unwrap_or(input);
    if hex.len() != 40 {
        return Err(format!("address must have 40 hex chars: {}", input));
    }

    let mut out = [0_u8; 20];
    for (i, slot) in out.iter_mut().enumerate() {
        let start = i * 2;
        let end = start + 2;
        let byte = u8::from_str_radix(&hex[start..end], 16)
            .map_err(|_| format!("invalid hex byte in address: {}", input))?;
        *slot = byte;
    }

    Ok(out)
}

fn format_address(address: &[u8; 20]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut formatted = String::with_capacity(42);
    formatted.push_str("0x");
    for byte in address {
        formatted.push(HEX[(byte >> 4) as usize] as char);
        formatted.push(HEX[(byte & 0x0f) as usize] as char);
    }
    formatted
}

fn compare_with_baseline(label: &str, baseline_path: &Path, generated: &str) -> Result<(), String> {
    if !baseline_path.exists() {
        return Err(format!(
            "missing baseline {} at {}",
            label,
            baseline_path.display()
        ));
    }

    let baseline = fs::read_to_string(baseline_path).map_err(|err| err.to_string())?;
    if baseline == generated {
        return Ok(());
    }

    Err(format!(
        "{} regression detected against {}\n{}",
        label,
        baseline_path.display(),
        diff_summary(&baseline, generated)
    ))
}

fn diff_summary(expected: &str, actual: &str) -> String {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();
    let max_lines = expected_lines.len().max(actual_lines.len());

    let mut lines = Vec::new();
    let mut mismatch_count = 0_usize;

    for index in 0..max_lines {
        let exp = expected_lines.get(index).copied().unwrap_or("<missing>");
        let act = actual_lines.get(index).copied().unwrap_or("<missing>");
        if exp != act {
            mismatch_count += 1;
            lines.push(format!(
                "line {}:\n  expected: {}\n  actual:   {}",
                index + 1,
                exp,
                act
            ));
            if mismatch_count >= 20 {
                lines.push("... truncated to first 20 differences ...".to_string());
                break;
            }
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{
        compare_with_baseline, execute_fixture, generate_reports, load_fixtures, parse_address,
        parse_u128, Fixture, FixtureBalance, FixtureExpected, FixtureTx,
    };
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(prefix: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "reth2030-vectors-{prefix}-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        fs::write(path, contents).expect("write file");
    }

    fn minimal_fixture_json(name: &str) -> String {
        format!(
            r#"{{
  "name": "{name}",
  "initial_accounts": [
    {{
      "address": "0x1111111111111111111111111111111111111111",
      "balance": "10"
    }}
  ],
  "transactions": [],
  "expected": {{
    "success": true,
    "balances": [
      {{
        "address": "0x1111111111111111111111111111111111111111",
        "balance": "10"
      }}
    ]
  }}
}}"#
        )
    }

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn no_op_fixture(name: &str, balance: &str) -> Fixture {
        Fixture {
            name: name.to_string(),
            initial_accounts: vec![FixtureBalance {
                address: "0x1111111111111111111111111111111111111111".to_string(),
                balance: balance.to_string(),
            }],
            transactions: Vec::new(),
            expected: FixtureExpected {
                success: true,
                balances: vec![FixtureBalance {
                    address: "0x1111111111111111111111111111111111111111".to_string(),
                    balance: balance.to_string(),
                }],
            },
        }
    }

    #[test]
    fn parse_address_accepts_20_byte_hex() {
        let address = parse_address("0x1111111111111111111111111111111111111111").expect("parse");
        assert_eq!(address, [0x11; 20]);
    }

    #[test]
    fn parse_u128_accepts_decimal_and_hex() {
        assert_eq!(parse_u128("42").expect("decimal"), 42);
        assert_eq!(parse_u128("0x2a").expect("hex"), 42);
        assert_eq!(parse_u128("0X2A").expect("uppercase hex"), 42);
    }

    #[test]
    fn parse_u128_rejects_invalid_values() {
        assert!(parse_u128("0x").is_err());
        assert!(parse_u128("not-a-number").is_err());
    }

    #[test]
    fn load_fixtures_recurses_into_nested_directories() {
        let temp_dir = TempDir::new("nested-fixtures");
        let nested_fixture_path = temp_dir.path.join("nested/fixtures/001.json");
        write_file(
            &nested_fixture_path,
            &minimal_fixture_json("nested-fixture"),
        );

        let fixtures = load_fixtures(&temp_dir.path).expect("load fixtures");
        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].name, "nested-fixture");
    }

    #[test]
    fn load_fixtures_rejects_duplicate_fixture_names() {
        let temp_dir = TempDir::new("duplicate-names");
        write_file(
            &temp_dir.path.join("a/001.json"),
            &minimal_fixture_json("duplicate-name"),
        );
        write_file(
            &temp_dir.path.join("b/001.json"),
            &minimal_fixture_json("duplicate-name"),
        );

        let err = load_fixtures(&temp_dir.path).expect_err("must reject duplicate names");
        assert!(err.contains("duplicate fixture name 'duplicate-name'"));
    }

    #[test]
    fn load_fixtures_rejects_unknown_fields() {
        let temp_dir = TempDir::new("unknown-fields");
        write_file(
            &temp_dir.path.join("001.json"),
            r#"{
  "name": "bad-fixture",
  "unexpected": true,
  "initial_accounts": [],
  "transactions": [],
  "expected": {
    "success": true,
    "balances": []
  }
}"#,
        );

        let err = load_fixtures(&temp_dir.path).expect_err("must reject unknown fields");
        assert!(err.contains("unknown field"));
        assert!(err.contains("unexpected"));
    }

    #[test]
    fn execute_fixture_reports_expected_failure_state() {
        let fixture = Fixture {
            name: "insufficient-balance".to_string(),
            initial_accounts: vec![FixtureBalance {
                address: "0x1111111111111111111111111111111111111111".to_string(),
                balance: "5".to_string(),
            }],
            transactions: vec![FixtureTx {
                from: "0x1111111111111111111111111111111111111111".to_string(),
                to: Some("0x2222222222222222222222222222222222222222".to_string()),
                nonce: 0,
                value: "6".to_string(),
            }],
            expected: FixtureExpected {
                success: false,
                balances: vec![
                    FixtureBalance {
                        address: "0x1111111111111111111111111111111111111111".to_string(),
                        balance: "5".to_string(),
                    },
                    FixtureBalance {
                        address: "0x2222222222222222222222222222222222222222".to_string(),
                        balance: "0".to_string(),
                    },
                ],
            },
        };

        let result = execute_fixture(&fixture).expect("fixture execution");
        assert!(result.passed);
        assert!(!result.actual_success);
    }

    #[test]
    fn execute_fixture_accepts_hex_numeric_fields() {
        let fixture = Fixture {
            name: "hex-transfer-success".to_string(),
            initial_accounts: vec![FixtureBalance {
                address: "0x1111111111111111111111111111111111111111".to_string(),
                balance: "0x1e".to_string(),
            }],
            transactions: vec![FixtureTx {
                from: "0x1111111111111111111111111111111111111111".to_string(),
                to: Some("0x2222222222222222222222222222222222222222".to_string()),
                nonce: 0,
                value: "0xa".to_string(),
            }],
            expected: FixtureExpected {
                success: true,
                balances: vec![
                    FixtureBalance {
                        address: "0x1111111111111111111111111111111111111111".to_string(),
                        balance: "0x14".to_string(),
                    },
                    FixtureBalance {
                        address: "0x2222222222222222222222222222222222222222".to_string(),
                        balance: "0xa".to_string(),
                    },
                ],
            },
        };

        let result = execute_fixture(&fixture).expect("fixture execution");
        assert!(result.passed);
        assert!(result.actual_success);
    }

    #[test]
    fn execute_fixture_flags_unexpected_post_state_accounts() {
        let fixture = Fixture {
            name: "missing-expected-recipient".to_string(),
            initial_accounts: vec![FixtureBalance {
                address: "0x1111111111111111111111111111111111111111".to_string(),
                balance: "10".to_string(),
            }],
            transactions: vec![FixtureTx {
                from: "0x1111111111111111111111111111111111111111".to_string(),
                to: Some("0x2222222222222222222222222222222222222222".to_string()),
                nonce: 0,
                value: "3".to_string(),
            }],
            expected: FixtureExpected {
                success: true,
                balances: vec![FixtureBalance {
                    address: "0x1111111111111111111111111111111111111111".to_string(),
                    balance: "7".to_string(),
                }],
            },
        };

        let result = execute_fixture(&fixture).expect("fixture execution");
        assert!(!result.passed);
        assert!(result
            .mismatches
            .iter()
            .any(|entry| entry.contains("unexpected account in post-state")));
    }

    #[test]
    fn execute_fixture_rejects_duplicate_expected_balances() {
        let fixture = Fixture {
            name: "duplicate-balance-expectation".to_string(),
            initial_accounts: vec![FixtureBalance {
                address: "0x1111111111111111111111111111111111111111".to_string(),
                balance: "10".to_string(),
            }],
            transactions: Vec::new(),
            expected: FixtureExpected {
                success: true,
                balances: vec![
                    FixtureBalance {
                        address: "0x1111111111111111111111111111111111111111".to_string(),
                        balance: "10".to_string(),
                    },
                    FixtureBalance {
                        address: "0x1111111111111111111111111111111111111111".to_string(),
                        balance: "10".to_string(),
                    },
                ],
            },
        };

        let err = execute_fixture(&fixture).expect_err("duplicate expected balances must fail");
        assert!(err.contains("duplicate expected balance entry"));
    }

    #[test]
    fn generate_reports_sorts_fixtures_by_name() {
        let fixtures = vec![no_op_fixture("zeta", "7"), no_op_fixture("alpha", "3")];
        let (scorecard, snapshot) = generate_reports(&fixtures).expect("generate reports");

        assert_eq!(scorecard.total, 2);
        assert_eq!(scorecard.passed, 2);
        assert_eq!(scorecard.failed, 0);
        assert_eq!(scorecard.pass_rate, 1.0);

        let names: Vec<&str> = snapshot
            .fixtures
            .iter()
            .map(|fixture| fixture.name.as_str())
            .collect();
        assert_eq!(names, vec!["alpha", "zeta"]);
    }

    #[test]
    fn generate_reports_handles_empty_fixture_list() {
        let fixtures = Vec::new();
        let (scorecard, snapshot) = generate_reports(&fixtures).expect("generate reports");

        assert_eq!(scorecard.total, 0);
        assert_eq!(scorecard.passed, 0);
        assert_eq!(scorecard.failed, 0);
        assert_eq!(scorecard.pass_rate, 0.0);
        assert!(snapshot.fixtures.is_empty());
    }

    #[test]
    fn compare_with_baseline_reports_line_level_diff() {
        let temp_dir = TempDir::new("baseline-diff");
        let baseline_path = temp_dir.path.join("baseline.json");
        write_file(&baseline_path, "{\n  \"value\": 1\n}\n");

        let err = compare_with_baseline("snapshot", &baseline_path, "{\n  \"value\": 2\n}\n")
            .expect_err("must detect baseline drift");
        assert!(err.contains("snapshot regression detected"));
        assert!(err.contains("line 2:"));
        assert!(err.contains("expected:   \"value\": 1"));
        assert!(err.contains("actual:     \"value\": 2"));
    }

    #[test]
    fn public_minimal_suite_matches_checked_in_baseline() {
        let root = workspace_root();
        let fixtures_dir = root.join("vectors/ethereum-state-tests/minimal");
        let baseline_scorecard = root.join("vectors/baseline/scorecard.json");
        let baseline_snapshot = root.join("vectors/baseline/snapshot.json");

        let fixtures = load_fixtures(&fixtures_dir).expect("load public suite fixtures");
        assert!(
            !fixtures.is_empty(),
            "public vector suite must include at least one fixture"
        );

        let (scorecard, snapshot) = generate_reports(&fixtures).expect("generate reports");
        let scorecard_json = serde_json::to_string_pretty(&scorecard).expect("serialize scorecard");
        let snapshot_json = serde_json::to_string_pretty(&snapshot).expect("serialize snapshot");

        compare_with_baseline("scorecard", &baseline_scorecard, &scorecard_json)
            .expect("scorecard baseline must match");
        compare_with_baseline("snapshot", &baseline_snapshot, &snapshot_json)
            .expect("snapshot baseline must match");
    }
}

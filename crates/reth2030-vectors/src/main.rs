use clap::Parser;
use reth2030_core::{Account, InMemoryState, StateStore};
use reth2030_types::{LegacyTx, Transaction};
use serde::{Deserialize, Serialize};
use std::{
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
struct Fixture {
    name: String,
    initial_accounts: Vec<FixtureBalance>,
    transactions: Vec<FixtureTx>,
    expected: FixtureExpected,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureBalance {
    address: String,
    balance: String,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureTx {
    from: String,
    to: Option<String>,
    nonce: u64,
    value: String,
}

#[derive(Debug, Clone, Deserialize)]
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

    let mut runs = Vec::with_capacity(fixtures.len());
    for fixture in &fixtures {
        runs.push(execute_fixture(fixture)?);
    }
    runs.sort_by(|a, b| a.name.cmp(&b.name));

    let passed = runs.iter().filter(|run| run.passed).count();
    let total = runs.len();
    let failed = total.saturating_sub(passed);
    let pass_rate = (passed as f64) / (total as f64);

    let scorecard = Scorecard {
        suite: "minimal-state-tests".to_string(),
        total,
        passed,
        failed,
        pass_rate,
    };

    let snapshot = SnapshotReport {
        suite: "minimal-state-tests".to_string(),
        fixtures: runs,
    };

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

fn load_fixtures(fixtures_dir: &Path) -> Result<Vec<Fixture>, String> {
    let mut paths = Vec::new();
    let entries = fs::read_dir(fixtures_dir).map_err(|err| err.to_string())?;

    for entry in entries {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            paths.push(path);
        }
    }

    paths.sort();

    let mut fixtures = Vec::with_capacity(paths.len());
    for path in paths {
        let contents = fs::read_to_string(&path).map_err(|err| err.to_string())?;
        let fixture: Fixture = serde_json::from_str(&contents)
            .map_err(|err| format!("failed to decode {}: {}", path.display(), err))?;
        fixtures.push(fixture);
    }

    Ok(fixtures)
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

    let mut actual_balances = Vec::with_capacity(fixture.expected.balances.len());
    for expected_balance in &fixture.expected.balances {
        let address = parse_address(&expected_balance.address)?;
        let expected_value = parse_u128(&expected_balance.balance)?;
        let actual_value = state
            .get_account(&address)
            .map(|account| account.balance)
            .unwrap_or(0);

        if actual_value != expected_value {
            mismatches.push(format!(
                "balance mismatch for {}: expected={}, actual={}",
                expected_balance.address, expected_value, actual_value
            ));
        }

        actual_balances.push(BalanceSnapshot {
            address: expected_balance.address.clone(),
            balance: actual_value.to_string(),
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
    value
        .parse::<u128>()
        .map_err(|_| format!("invalid numeric value: {}", value))
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
        execute_fixture, parse_address, Fixture, FixtureBalance, FixtureExpected, FixtureTx,
    };

    #[test]
    fn parse_address_accepts_20_byte_hex() {
        let address = parse_address("0x1111111111111111111111111111111111111111").expect("parse");
        assert_eq!(address, [0x11; 20]);
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
}

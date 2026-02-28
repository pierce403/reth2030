# Conformance and Regression Workflow

This project uses `reth2030-vectors` to execute a minimal public fixture suite and
track conformance progress over time.

## Commands

```bash
# Run fixture suite and compare against baseline
cargo run -p reth2030-vectors -- \
  --fixtures-dir vectors/ethereum-state-tests/minimal \
  --baseline-scorecard vectors/baseline/scorecard.json \
  --baseline-snapshot vectors/baseline/snapshot.json \
  --out-dir artifacts/vectors

# Intentionally refresh baseline after approved behavior changes
cargo run -p reth2030-vectors -- --update-baseline
```

## Outputs

- `artifacts/vectors/scorecard.json`
- `artifacts/vectors/snapshot.json`

The scorecard tracks:
- total fixtures
- passed fixtures
- failed fixtures
- pass rate (`0.0..1.0`)

## Conformance Metric Timeline

To make progress measurable over time, this repo keeps an append-only history file at:

- `vectors/baseline/conformance-history.json`

The tracked metric is `minimal-state-tests-pass-rate` for the
`minimal-state-tests` suite. Each history entry records:

- `recorded_on` (`YYYY-MM-DD`, strictly increasing)
- `total`, `passed`, `failed`
- `pass_rate`

When a baseline update is intentionally approved, append a new entry that matches
the new `vectors/baseline/scorecard.json` values.
Do not edit or remove prior entries; preserve historical records and only append.

Fixture discovery and parsing rules:
- JSON fixtures are discovered recursively under the configured fixtures directory.
- Numeric fields (`balance`, `value`) accept decimal strings or `0x`-prefixed hex strings.
- Fixture schema is strict (`deny_unknown_fields`) to catch accidental drift early.

## Triage Workflow

1. If CI fails with a scorecard or snapshot regression, inspect the diff summary in logs.
2. Open `artifacts/vectors/snapshot.json` to find fixture-level mismatch details.
3. Determine whether behavior changed intentionally or due to a regression.
4. For intended changes, update fixture expectations and run `--update-baseline`.
5. If baseline changed intentionally, append a matching entry to
   `vectors/baseline/conformance-history.json`.
6. For unintended changes, fix implementation and keep baseline/history unchanged.

## Known Deviations (Current)

- The vector harness currently uses simplified Legacy transaction execution only.
- Gas accounting is placeholder-level, not fork-accurate.
- State transitions are deterministic but intentionally minimal versus full EF semantics.

These are expected limitations for the current POC stage.

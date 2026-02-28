# AGENTS.md - Instructions for Coding Agents

## Self-Improvement Directive

Every coding agent working on this repository must update this file whenever
important project knowledge is learned.

Capture both:
- successes to repeat,
- failures to avoid,
- collaborator preferences that improve teamwork.

Do this before ending a task when new information was discovered.

## Project Overview

`reth2030` is a Rust execution-client scaffold inspired by ETH2030.
This repository currently contains:
- a workspace binary crate: `crates/reth2030`
- a workspace core library crate: `crates/reth2030-core`

Current status: experimental scaffold, not production-ready.

## Build and Test Commands

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p reth2030 -- --help
cargo run -p reth2030 -- --chain sepolia
cargo run -p reth2030-vectors -- --fixtures-dir vectors/ethereum-state-tests/minimal --baseline-scorecard vectors/baseline/scorecard.json --baseline-snapshot vectors/baseline/snapshot.json --out-dir artifacts/vectors
```

## Coding Conventions

- Use Rust workspace layout and keep crates focused by responsibility.
- Keep public interfaces small and explicit.
- Use descriptive commit messages tied to milestones.
- Prefer deterministic tests with clear assertions.
- Use Apache-2.0 licensing metadata for new crates/files and keep docs aligned.

## Known Issues and Solutions

- HTTPS git push may fail without credential helper in this environment.
  Solution: use SSH remote URL (`git@github.com:...`) for push operations.
- Cargo commands may temporarily block on build directory lock when run in
  parallel sessions.
  Solution: allow one command to finish or run sequentially.

## Agent Tips

- Keep all external reference clones under `code/` only.
- `code/` is git-ignored except `code/README.md`.
- Verify `git status --short` before each commit.
- After each milestone, commit and push immediately.
- Whenever a task changes TODO milestones/checklists, update `index.html`
  status language in the same change.
- Never claim milestone readiness/completion on `index.html` unless explicitly
  re-verified against current `TODO.md` and repository state; default to
  conservative "in progress" wording.
- Use `./ralph.sh` for autonomous iteration loops; it logs progress to
  `ralph.log` and runs Codex with full permissions.
- `ralph.sh` now defaults to a 2-hour wall-clock budget via
  `RALPH_MAX_RUNTIME_SECONDS=7200` (`0` disables); use this to avoid runaway
  autonomous loops.
- `ralph.sh` should prefer small TODO items and treat compound tasks as too
  large; if only large tasks remain, break the chosen task into 5-7 smaller
  unchecked TODO items before implementation.

## Rapport and Reflection Notes

Collaborator preferences observed:
- Commit early and commit often.
- Push after every milestone/task chunk.
- Keep communication concise and execution-focused.
- Keep external reference code out of main tracked source tree.

Reflection cadence:
- Revisit this file whenever workflow friction appears.
- Consolidate repeated guidance into short, actionable bullets.
- Keep this document concise but specific.

## Learning Log

### 2026-02-28

- Established policy: external reference code must live under `code/`.
- Removed an out-of-tree upstream reference clone to keep scope clean.
- Bootstrapped a 2-crate Rust workspace (`reth2030`, `reth2030-core`).
- Added baseline CLI and config APIs (`Chain`, `NodeConfig::default_for`).
- Confirmed milestone-based commit/push workflow as team standard.
- Added CI at `.github/workflows/ci.yml` with fmt/check/test/clippy gates.
- Added optional pre-commit hooks via `.pre-commit-config.yaml`.
- Added contributor onboarding in `CONTRIBUTING.md`.
- Added ADR docs under `docs/architecture-notes/` (starting at ADR-0001).
- Added `reth2030-types` crate with canonical tx/header/block/receipt structs.
- Added JSON boundary helpers and explicit `u128` string encoding.
- Added `StateStore` trait plus deterministic `InMemoryState` transitions/tests.
- Learned pitfall: JSON round-trip of raw `u128` fails unless explicitly encoded.
- Added `ExecutionEngine` abstraction and `SimpleExecutionEngine` scaffold.
- Added deterministic execution outputs (`TxExecutionResult`, receipts, gas totals).
- Added integration tests in `crates/reth2030-core/tests/execution_ordering.rs`.
- Learned pitfall: pre-execution blocks may have empty receipts; validation must allow that.
- Added `reth2030-rpc` crate with Axum router and JSON-RPC request/response types.
- Implemented baseline methods: `web3_clientVersion`, `eth_chainId`, `eth_blockNumber`.
- Added `/engine` route with placeholder bearer-JWT guard and structured auth errors.
- Added API-level tests using `tower::ServiceExt::oneshot` for success/error shape checks.
- Added `reth2030-net` crate with peer/session lifecycle primitives.
- Added `SyncOrchestrator` pipeline (`headers -> bodies -> execution`) with mock source/sink.
- Added deterministic integration tests in `crates/reth2030-net/tests/sync_orchestration.rs`.
- Added node startup/shutdown orchestration plus `--run-mock-sync` in `reth2030` CLI.
- Added `reth2030-vectors` crate for fixture execution and conformance reporting.
- Added minimal public fixture suite under `vectors/ethereum-state-tests/minimal`.
- Added baseline regression files under `vectors/baseline/` with diff-based detection.
- Added CI artifact publishing for vector reports (`artifacts/vectors`) in workflow.
- Added root `index.html` project site with ETH2030 references and milestone board.
- Added process rule: task completion requires corresponding website progress update.
- Added `ralph.sh` automation loop for random TODO-driven Codex execution.
- Added `ralph.log` ignore rule to keep loop logs out of git history.
- Standardized project licensing to Apache-2.0 (root LICENSE + crate manifests).
- Hardened `reth2030-vectors` fixture loading to recurse subdirectories deterministically.
- Learned pitfall: duplicate fixture `name` values across files caused ambiguous reporting; now rejected at load time.
- Hardened fixture schema parsing with `deny_unknown_fields` to surface format drift early.
- Added decimal + `0x` hex numeric parsing for vector balances/values to better match Ethereum-style fixtures.
- Hardened CI by splitting vector conformance into its own `vector-conformance` job so public-suite execution is attempted on every CI run.
- Added a vectors unit test that replays `vectors/ethereum-state-tests/minimal` and asserts checked-in baseline parity during `cargo test --workspace`.
- Hardened Engine API auth placeholder parsing: reject duplicate/malformed `Authorization` headers and fail closed when server JWT config is blank.
- Hardened Engine API flow to enforce auth before request-body decoding, reducing unauthenticated parse surface on `/engine`.
- Learned pitfall: adding optional structured JSON-RPC error data can trigger `clippy::result_large_err`; boxing large `Err` variants keeps `-D warnings` green.
- Hardened `SimpleExecutionEngine` gas accounting: reject txs when intrinsic gas exceeds per-tx `gas_limit` and detect `u64` cumulative gas overflow explicitly.
- Learned pitfall: receipt pseudo-hash based only on `(from, nonce, index)` can collide for distinct tx content; hash derivation now includes full tx payload and fee fields.
- Hardened sync orchestration to fail closed on malformed header batches: reject over-limit responses, non-contiguous/duplicate header numbers, and `u64` sequence overflow before fetching bodies or executing blocks.
- Added deterministic malformed-source sync tests covering limit violations, missing start header, gaps, duplicates, overflow, zero-limit no-op, and partial execution before failure.
- Hardened CI workflow with least-privilege permissions, concurrency cancellation, lockfile-enforced cargo commands, and job timeouts.
- Added `ci_workflow_contract` tests that parse `.github/workflows/ci.yml` and fail fast on trigger/gate/artifact drift.
- Learned pitfall: representing toolchain `components` as a comma-delimited scalar is less robust for strict YAML contract testing than an explicit sequence.
- Learned pitfall: website milestone boards can overstate project readiness; use conservative status language and avoid blanket completion claims unless freshly verified.
- Hardened `ralph.sh` with a default 2-hour runtime limit and timeout-backed iteration cutoff to prevent unbounded autonomous execution.
- Added small-task-first `ralph.sh` selection heuristics and a fallback rule to decompose oversized TODO items into 5-7 concrete subtasks.
- Hardened `reth2030-types` serde boundaries with `deny_unknown_fields` on core protocol structs to fail fast on schema drift.
- Added exhaustive `reth2030-types` tests for all tx variant accessors, `u128` JSON edge-cases (max values, invalid forms, numeric back-compat), and block/header validation ordering.
- Added `contributor_docs_contract` integration tests to keep `CONTRIBUTING.md` and `README.md` core-check commands present, ordered, and fenced for copy/paste reliability.
- Learned pitfall: `README.md` Quick Start can drift from contributor quality gates (it was missing the `cargo clippy --workspace --all-targets -- -D warnings` command).
- Added `reth2030` runtime tests covering `run_mock_sync_once` success, fail-closed behavior at `max_peers=0`, and repeated-run stability.
- Learned behavior: mocked sync reuses a stable peer ID; repeated runs should stay non-panicking and keep peer count bounded while appending observable peer events.
- Added `architecture_notes_contract` tests that enforce ADR directory presence, strict `ADR-####-slug.md` naming, contiguous numbering, and required ADR sections.
- Learned pitfall: TODO seed tasks can already be checked; convert those items into contract tests to keep the completed state continuously enforced.
- Hardened crate-level API documentation with `crate_api_docs_contract` tests that require each library crate to maintain an explicit `## Public API` symbol list matching expected exports.
- Hardened `reth2030-types` block validation so populated receipts must have non-decreasing `cumulative_gas_used` and a final value equal to `header.gas_used`.
- Learned pitfall: receipt-bearing block fixtures can silently drift from header gas accounting unless receipt cumulative gas invariants are validated explicitly.
- Added `vectors/baseline/conformance-history.json` as an append-only conformance metric timeline tied to `vectors/baseline/scorecard.json`.
- Added `conformance_history_contract` tests to enforce valid date ordering, score math invariants, and latest-entry parity with the checked-in scorecard.
- Hardened `reth2030` runtime lifecycle with explicit `Initialized/Running/Stopped` transitions and fail-closed start/run/shutdown guards.
- Learned behavior: shutdown should disconnect all connected peers deterministically, and `main` should attempt shutdown even when mock sync fails.

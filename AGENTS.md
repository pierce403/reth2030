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
- Hardened `ExecutionEngine` trait boundary to be object-safe (`&mut dyn StateStore`) so backends can be swapped behind `dyn ExecutionEngine`.
- Learned pitfall: generic trait methods on `ExecutionEngine` block trait-object dispatch and weaken runtime pluggability guarantees.
- Added explicit `PeerSession` abstraction in `reth2030-net` with monotonic session IDs managed by `PeerManager`.
- Learned behavior: reconnecting an existing peer rotates its session in-place (new `session_id`) without consuming an additional peer slot.
- Learned pitfall: session ID allocation should fail closed on `u64` overflow before mutating peer/session state or emitting lifecycle events.
- Hardened `architecture_notes_contract` ADR content checks to enforce first-line ADR title numbering, single metadata lines, allowed status values, ISO `YYYY-MM-DD` dates, and non-empty ordered required sections.
- Added explicit peer observability stubs in `PeerManager`: deterministic lifecycle log lines and metrics snapshots (`connected`, `disconnected`, `rejected_max_peers`, `active_peers`).
- Learned behavior: clearing the transient `PeerEvent` buffer should not reset log/metrics stubs, preserving lifecycle observability across event-polling boundaries.
- Hardened `SyncOrchestrator::run_once` so `limit=0` short-circuits before `SyncSource::fetch_headers`, ensuring zero-limit sync tests never touch external fetch hooks.
- Added sync integration assertions that malformed header batches trigger fail-closed rejection before any body fetch and that execution failure stops further body fetches immediately.
- Hardened contributor command contracts to require a single core-check-prefix fenced block in `README.md` and `AGENTS.md`, reducing onboarding ambiguity.
- Learned pitfall: order-only docs checks can miss ambiguous/duplicate command snippets; enforce unique prefix matches for contributor-facing check commands.
- Added exhaustive `InMemoryState` transition tests for storage write semantics: writes lazily create default accounts and key overwrites remain account-scoped.
- Learned behavior: failed `transfer` calls are atomic (no sender/recipient mutation), while self-transfers preserve balance and only bump sender nonce.
- Hardened `reth2030-rpc` JSON-RPC request validation: reject non-spec `id` types (bool/array/object) and scalar `params`, while still allowing omitted `params`.
- Learned pitfall: modeling request `id`/`params` as unconstrained `serde_json::Value` silently accepts invalid JSON-RPC shapes unless explicitly validated.
- Added repeated-run execution determinism tests in `reth2030-core` covering mixed tx variants (`Legacy`, `Eip1559`, `Blob`) with payload/blob fields.
- Learned behavior: execution failures after partial state progress are deterministic across reruns from identical pre-state (stable `ExecutionError` and identical post-failure state snapshot).
- Learned workflow: when `ralph.sh` picks an already-checked seed task, add a focused contract test that locks the completed outcome (TODO checkmark + workspace/member/API invariants) instead of reopening TODO state.
- Added `public_vector_ci_seed_contract` tests to lock the checked CI vector-suite seed outcome (`TODO.md` checkmark + automatic workflow triggers + `vector-conformance` job contract + public fixture presence).
- Learned pitfall: a job-level `if` on `vector-conformance` can silently bypass automatic suite execution even when workflow triggers are correct; keep the job ungated by default.
- Added `minimal_state_test_subset_seed_contract` tests to lock the checked TODO seed for integrating a minimal Ethereum state-test subset (canonical fixture paths, mixed success/failure coverage, ordering-sensitive failure shape, and hex-value fixture coverage).
- Learned pitfall: vector subset integration can drift without a task-specific contract, even when broader vector CI and baseline checks still pass.
- Hardened `public_vector_ci_seed_contract` with path-filter checks and explicit guarantees that `vector-conformance` has no `needs`, no step-level gating, and no `continue-on-error` masking.
- Learned pitfall: even with correct workflow triggers, push/PR path filters or step-level `continue-on-error` can silently weaken the "public vector suite runs automatically in CI" guarantee.
- Added `reth2030_rpc_seed_contract` tests to lock the checked RPC skeleton seed outcome (`TODO.md` checkmark + workspace member + crate manifest + router/server wiring invariants).
- Learned pitfall: without a seed-specific contract, basic RPC skeleton guarantees (root/engine route topology and HTTP server entrypoint wiring) can drift while unrelated RPC behavior tests still pass.
- Added `mock_sync_seed_contract` tests to lock the checked Phase 4 seed outcome (`TODO.md` checkmark + mocked-sync CLI/runtime wiring + runtime non-panicking/fail-closed test coverage contract).
- Learned pitfall: mocked sync readiness can drift if `main.rs` test coverage is slimmed down; enforce presence of success, fail-closed, and repeated-run runtime tests via a seed contract.
- Added `minimal_executable_block_flow_seed_contract` tests to lock the checked Phase 1 acceptance criterion (`TODO.md` checkmark + pre-execution empty-receipt representability + post-execution receipt/gas invariant boundaries).
- Learned pitfall: checked acceptance criteria can drift without explicit contract coverage, even when lower-level unit tests still pass.
- Added `startup_shutdown_seed_contract` tests to lock the checked Phase 4 startup/shutdown task (`TODO.md` checkmark + lifecycle state machine wiring + runtime test coverage contract).
- Learned behavior: orchestration should still attempt `shutdown` after a mock-sync failure, leaving runtime state fail-closed at `Stopped`.
- Hardened `reth2030-types` serde boundary tests to reject unknown fields across all transaction variants and core structs (`Header`, `Receipt`, `LogEntry`, `Block`).
- Learned behavior: `u128` string fields accept `u128::MAX` values across tx variants and fail closed on overflow strings.
- Added `execution_engine_seed_contract` tests to lock the checked Phase 2 seed outcome (`TODO.md` checkmark + `ExecutionEngine` trait signature/object-safety + crate re-export + dyn-dispatch contract).
- Learned pitfall: even when execution behavior tests exist, a checked seed task can still drift without a dedicated seed contract tied to the exact TODO line and API boundary.
- Added `block_execution_pipeline_seed_contract` tests to lock the checked Phase 2 acceptance criterion (`TODO.md` checkmark + execution pipeline source wiring + end-to-end in-process success/fail-closed behavior).
- Learned pitfall: source-fragment contract assertions can be overly formatting-sensitive around method chaining; prefer whitespace-normalized, token-stable fragment checks.
- Hardened block execution pipeline seed coverage with a block-gas-limit edge case: when cumulative gas would exceed `header.gas_limit`, execution fails before applying the offending tx while preserving prior successful tx state.
- Hardened `public_vector_ci_seed_contract` further to fail closed on additional CI-bypass patterns: disallow `push` branch/tag ignore filters, require `pull_request.types` to keep `opened`/`reopened`/`synchronize` when present, and require `pull_request.branches` to include `main` when specified.
- Learned pitfall: even with a valid vector command and ungated job, automatic CI guarantees can still be weakened by job-level `continue-on-error`, non-upload step `if` guards, or shell fragments like `|| true`/`set +e` that mask vector failures.
- Hardened `minimal_state_test_subset_seed_contract` with fixture arithmetic/order invariants (successful transfer conservation, fail-closed insufficient-balance behavior, and partial-progress ordering failure semantics).
- Learned pitfall: checking only fixture presence/name/success flags is insufficient; minimal subset contracts should also lock decimal/hex encoding mix and transfer-balance arithmetic to prevent semantic drift.
- Added `state_transition_seed_contract` tests to lock the checked Phase 1 account/storage transition seed (`TODO.md` checkmark + required state-unit coverage + fail-closed missing-sender transfer behavior).
- Hardened `InMemoryState::transfer` fail-closed semantics by avoiding sender account creation on insufficient-balance errors for previously missing senders.
- Learned pitfall: using `entry(...).or_default()` before insufficient-balance validation can silently mutate state on failed transfers; validate first, then persist mutations.
- Hardened `minimal_executable_block_flow_seed_contract` with mixed transaction variant coverage (`Legacy`, `Eip1559`, `Blob`) for pre/post-execution block shapes plus header gas-limit fail-closed precedence checks.
- Learned pitfall: minimal-flow seed coverage that only exercises legacy transactions can miss representability drift in non-legacy tx variants and validation-order guarantees.
- Added `peer_lifecycle_observability_seed_contract` tests to lock the checked Phase 4 acceptance criterion (`TODO.md` checkmark + peer observability wiring + required unit/integration coverage for logs/metrics stubs).
- Learned workflow: when an acceptance criterion is already checked but not explicitly guarded, add a focused seed contract that asserts both API wiring and edge-case test presence to prevent silent observability drift.
- Added `scaffold_engine_seed_contract` tests to lock the checked Phase 2 task for scaffold execution (`TODO.md` checkmark + `SimpleExecutionEngine` wiring + runtime coverage for empty/no-op/custom intrinsic-gas paths).
- Learned behavior: `SimpleExecutionEngine::new(0)` provides a no-op gas-accounting mode while still applying state transitions, so scaffold coverage should assert both zero-gas results and state mutation.
- Added `reth2030_net_seed_contract` tests to lock the checked Phase 4 seed outcome (`TODO.md` checkmark + workspace member + crate manifest + peer/session abstraction wiring + edge-case unit-test coverage contract).
- Learned behavior: `PeerManager::new(0)` must fail closed on first connect while still emitting rejected-peer observability signals (event/log/metrics).
- Learned behavior: reconnecting the same peer ID rotates `session_id` and replaces the stored peer address in-place.
- Hardened `reth2030-types` serde boundary coverage with invalid fixed-length byte tests across tx/header/receipt/log decoding (`Address`/`Hash32` fields).
- Learned pitfall: serde invalid-length error text varies between underflow and overflow cases; assert semantic failure (`invalid length`) instead of exact phrasing.
- Hardened `minimal_executable_block_flow_seed_contract` with contract-creation coverage (`to: None`) across `Legacy`, `Eip1559`, and `Blob` transaction variants.
- Learned behavior: block-flow validation intentionally allows equal adjacent `receipt.cumulative_gas_used` values (non-decreasing, not strictly increasing), so seed tests should lock this with a plateau-gas case.
- Learned behavior: max-peer rejections occur before session allocation, so rejected new-peer connects must not consume `PeerManager` session IDs.
- Learned behavior: runtime shutdown disconnect ordering is deterministic by `PeerId` key order even when peers were connected out of order, so startup/shutdown tests should lock that sequence explicitly.
- Learned behavior: `NodeRuntime::execute` fails closed without lifecycle mutation if called while already running (start precondition failure path).
- Added `execution_determinism_seed_contract` tests to lock the checked Phase 2 acceptance criterion (`TODO.md` checkmark + repeated-run determinism across success and fail-closed error paths).
- Learned behavior: repeated runs are deterministic for mixed contract-creation execution and for fail-closed intrinsic-gas/gas-limit failures, including identical partial post-state snapshots.
- Added `conformance_metric_seed_contract` tests to lock the checked Phase 5 acceptance criterion (`TODO.md` checkmark + multi-entry conformance timeline invariants + latest-entry scorecard parity + docs append-only guidance).
- Learned pitfall: a single history snapshot does not satisfy "tracked over time"; require at least two strictly increasing `recorded_on` entries in `vectors/baseline/conformance-history.json`.
- Hardened `block_execution_pipeline_seed_contract` to require intrinsic per-tx gas-limit validation wiring and to assert error-precedence behavior when intrinsic and block-gas guards would both fail.
- Learned behavior: intrinsic gas validation must fail before block gas-limit accounting for the same tx, and the failing tx must remain fail-closed while earlier successful tx state progress is preserved.
- Hardened `scaffold_engine_seed_contract` with mixed-variant no-op coverage (`Legacy`/`Eip1559`/`Blob` + contract-creation) under `SimpleExecutionEngine::new(0)`.
- Learned behavior: with scaffold intrinsic gas set to zero, block execution can run with `tx.gas_limit=0` and `header.gas_limit=0` while still applying deterministic state transitions across mixed transaction variants.
- Learned behavior: when `base_gas_per_tx` exceeds `header.gas_limit` on the first transaction, `SimpleExecutionEngine` fails closed with `GasLimitExceeded` before any state mutation.
- Hardened `conformance_metric_seed_contract` to lock bootstrap timeline continuity: the first `conformance-history` entry (2026-02-27, 3/4 pass) is now immutable while newer entries append.
- Learned pitfall: without a fixed bootstrap-entry anchor, append-only conformance timelines can be silently rewritten while still passing monotonic-date and latest-scorecard parity checks.

### 2026-03-01

- Hardened `execution_engine_seed_contract` with dyn-dispatch edge coverage: `&dyn ExecutionEngine` helper callsites, external engine implementations, and invalid-block error propagation through trait objects.
- Learned pitfall: state-focused engine contract tests must use the `StateStore` surface (`get_account`/`upsert_account`) and account for `Account::storage` in initializers to match current core APIs.
- Hardened `conformance_metric_seed_contract` with arithmetic edge coverage so history entries fail closed when `passed + failed` overflows `u64`.
- Learned pitfall: scorecard/history mismatch tests should isolate the `pass_rate` comparison path directly; otherwise field-level drift can mask regressions in the dedicated pass-rate guard.
- Hardened `public_vector_ci_seed_contract` to enforce fail-closed vector command structure (single cargo invocation, argument-only continuations, and disallowed shell control/masking fragments) plus strict `upload-artifact` uniqueness/`if: always()` guarantees.
- Learned pitfall: naive shell-keyword substring guards (for example `fi`) can false-positive against valid vector arguments like `--fixtures-dir`; prefer boundary-aware or contextual shell-fragment checks.
- Hardened `public_vector_ci_seed_contract` to require `vector-conformance` remain free of job-level `strategy`/`matrix` indirection, preventing zero-expansion configurations from silently skipping the suite.
- Learned pitfall: shell-keyword fail-closed checks should validate token boundaries (for example `if`/`then`) so safe argument substrings (for example `office-thenable`) do not trigger false positives.
- Hardened mocked-sync runtime coverage for peer-slot saturation by a different peer: repeated `run_mock_sync_once` attempts fail closed (`MaxPeersReached`) without mutating the connected peer, and the loop recovers deterministically once the slot is freed.
- Hardened `mock_sync_seed_contract` so this saturation/recovery runtime test remains required and the checked seed acceptance does not silently regress.
- Hardened startup/shutdown runtime coverage for stopped-state re-entry: calling `execute(...)` after a completed execute must fail closed at `start` and preserve prior peer events/logs/metrics without mutation.
- Learned behavior: once `NodeRuntime` reaches `Stopped`, further `execute(...)` calls are intentionally non-restartable and should leave observability state unchanged.
- Added `vector_harness_seed_contract` tests to lock the checked Phase 5 seed outcome (`TODO.md` checkmark + workspace/member + vectors crate manifest + fixture-harness pipeline wiring + edge-case unit-test coverage contract).
- Learned pitfall: CI vector-suite contract coverage alone does not guarantee the vector harness seed remains intact; keep a dedicated seed contract tied to the exact TODO line and harness source/test invariants.
- Added `state_backend_determinism_seed_contract` tests to lock the checked Phase 1 acceptance criterion (`TODO.md` checkmark + deterministic transition test-presence contract + replay parity on success and fail-closed paths).
- Hardened `InMemoryState` deterministic transition coverage with replay edge cases for partial-progress failure, contract-creation sender-only mutation, and interleaved storage writes.
- Learned pitfall: deterministic transition coverage must validate both error equality and post-state snapshot parity; success-only replay assertions can miss fail-closed drift.
- Added `engine_api_namespace_seed_contract` tests to lock the checked Phase 3 task for Engine API namespace/JWT placeholders (`TODO.md` checkmark + `/engine` auth wiring + engine capability/placeholder method contract + auth edge-case coverage contract).
- Learned pitfall: source-level constant-array contract parsers must anchor on initializer `=` before scanning `[...]`; starting at the type annotation bracket (`[&str; N]`) can silently drop first entries.

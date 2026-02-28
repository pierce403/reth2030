# TODO - reth2030 Roadmap

This file tracks implementation work in executable phases.

## Phase 0 - Repo Hygiene and Tooling

### Goals
- Keep the repository easy to navigate and reliable for contributors.
- Establish baseline CI and development workflow quality gates.

### Tasks
- [x] Add GitHub Actions CI for `fmt`, `check`, `test`, and `clippy`.
- [x] Add pre-commit hooks (optional) for local `fmt` and lint.
- [x] Add `CONTRIBUTING.md` with setup and coding workflow.
- [x] Add architecture-notes directory for ADR-style decisions.

### Acceptance Criteria
- [x] PRs run CI checks automatically and block on failure.
- [x] New contributors can run all core checks using documented commands.
- [x] Build and test steps are reproducible from a clean checkout.

### Dependencies
- Depends on: none.
- Unblocks: all later phases by reducing integration friction.

## Phase 1 - Execution-Layer Data Types and State Model

### Goals
- Introduce canonical block, transaction, and receipt types.
- Define state interfaces with a first local backend.

### Tasks
- [x] Create `reth2030-types` crate for primitive protocol types.
- [x] Define transaction variants and serialization boundaries.
- [x] Define block/header/receipt types and validation helpers.
- [x] Add a `StateStore` trait and in-memory implementation.
- [x] Add state transition unit tests for basic account/storage updates.

### Acceptance Criteria
- [x] Types can represent at least a minimal executable block flow.
- [x] State backend passes deterministic transition tests.
- [x] Public APIs are documented at crate-level.

### Dependencies
- Depends on: Phase 0 quality gates.
- Unblocks: EVM execution integration and RPC schema wiring.

## Phase 2 - EVM Execution Adapter Strategy

### Goals
- Execute transactions against the state model with clear boundaries.
- Allow replacement of execution backend without rewiring the whole node.

### Tasks
- [x] Define an `ExecutionEngine` trait in core.
- [x] Implement a scaffold engine (no-op or simplified execution path).
- [x] Add gas accounting placeholders and explicit TODO markers for fork rules.
- [x] Add deterministic execution-result types (status, gas used, logs).
- [x] Add integration tests for multi-transaction block execution ordering.

### Acceptance Criteria
- [x] A block execution pipeline exists end-to-end in-process.
- [x] Execution output is deterministic under repeated runs.
- [x] Engine abstraction boundary is stable and documented.

### Dependencies
- Depends on: Phase 1 types + state model.
- Unblocks: RPC/Engine API and sync pipeline scaffolding.

## Phase 3 - JSON-RPC and Engine API Skeleton

### Goals
- Expose core node capabilities through a stable API surface.
- Prepare execution/consensus interface points.

### Tasks
- [x] Create `reth2030-rpc` crate with HTTP JSON-RPC server skeleton.
- [x] Implement baseline methods (`web3_clientVersion`, `eth_chainId`, `eth_blockNumber`).
- [x] Add Engine API namespace skeleton with JWT auth placeholders.
- [x] Define request/response types and error mapping strategy.
- [x] Add API-level tests for success and structured error responses.

### Acceptance Criteria
- [x] Server starts and serves documented baseline RPC methods.
- [x] Engine API routes are wired and guarded by auth placeholders.
- [x] API tests run in CI and validate stable response shapes.

### Dependencies
- Depends on: Phase 1 data types and Phase 2 execution outputs.
- Unblocks: network sync and external client interoperability tests.

## Phase 4 - Networking and Sync Scaffolding

### Goals
- Build minimal P2P plumbing and chain sync orchestration.
- Ensure node lifecycle management can host future protocol features.

### Tasks
- [x] Create `reth2030-net` crate for peer/session abstractions.
- [x] Add a basic sync pipeline interface (headers -> bodies -> execution).
- [x] Add peer management scaffolding (connect, disconnect, peer limits).
- [x] Add startup/shutdown orchestration in `reth2030` binary.
- [x] Add integration tests with mocked peers and deterministic responses.

### Acceptance Criteria
- [x] Node can run a mocked sync loop without panic.
- [x] Peer lifecycle events are observable through logs/metrics stubs.
- [x] Sync orchestration is testable without external networks.

### Dependencies
- Depends on: Phase 2 execution and Phase 3 API surfaces.
- Unblocks: conformance testing and external interoperability.

## Phase 5 - Test Vectors and Conformance Path

### Goals
- Build confidence with repeatable vector-driven validation.
- Establish objective progress against compatibility targets.

### Tasks
- [x] Add vector harness crate or module for fixture execution.
- [x] Integrate a minimal Ethereum state-test subset first.
- [x] Add snapshot and regression reporting outputs.
- [x] Document pass/fail triage workflow and known deviations.
- [x] Define compatibility scorecard and publish in CI artifacts.

### Acceptance Criteria
- [x] At least one public vector suite runs automatically in CI.
- [x] Regressions are detected and surfaced with actionable diffs.
- [x] A documented conformance metric is tracked over time.

### Dependencies
- Depends on: Phases 1-4 functional scaffolding.
- Unblocks: iterative hardening and feature parity tracking.

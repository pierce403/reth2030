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
```

## Coding Conventions

- Use Rust workspace layout and keep crates focused by responsibility.
- Keep public interfaces small and explicit.
- Use descriptive commit messages tied to milestones.
- Prefer deterministic tests with clear assertions.

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

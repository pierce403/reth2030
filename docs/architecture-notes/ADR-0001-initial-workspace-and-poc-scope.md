# ADR-0001: Initial Workspace and POC Scope

- Status: Accepted
- Date: 2026-02-28

## Context

`reth2030` starts as an experimental Rust implementation inspired by ETH2030.
We need fast iteration, clear module boundaries, and repeatable quality checks.

## Decision

- Use a Rust workspace as the top-level project structure.
- Keep an initial split of:
  - `reth2030` binary for CLI and process orchestration.
  - `reth2030-core` for shared types/config and execution abstractions.
- Treat higher-level protocol work (RPC, networking, conformance) as separate
  crates to preserve dependency hygiene.
- Keep the first POC centered on local deterministic behavior, not real network
  consensus interoperability.

## Consequences

Positive:
- Faster onboarding and lower refactor pressure as scope expands.
- Easier unit and integration testing with clean seams.
- Clear path for adding `reth2030-types`, `reth2030-rpc`, and `reth2030-net`.

Tradeoffs:
- More crate boilerplate earlier.
- Extra coordination for cross-crate interface changes.

# reth2030

Rust execution client scaffold inspired by [ETH2030](https://eth2030.com/).

Status: experimental scaffold in active development.

## Workspace Layout

- `crates/reth2030` - CLI binary (`reth2030`)
- `crates/reth2030-core` - shared core types and config scaffolding
- `crates/reth2030-net` - peer/session and sync orchestration scaffolding
- `crates/reth2030-rpc` - JSON-RPC + Engine API server skeleton
- `crates/reth2030-types` - execution-layer primitive types (tx, block, receipt)
- `crates/reth2030-vectors` - fixture harness and conformance scorecard generator
- `code/` - local external reference code (ignored by git except `code/README.md`)

## Quick Start

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p reth2030 -- --help
cargo run -p reth2030 -- --chain sepolia
cargo run -p reth2030 -- --run-mock-sync
cargo run -p reth2030-vectors -- --update-baseline
```

## Notes

This repository is intentionally starting with minimal interfaces so protocol
components can be added incrementally with clear milestones.

## License

This project is licensed under the Apache License 2.0. See `LICENSE`.

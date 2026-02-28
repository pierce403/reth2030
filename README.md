# reth2030

Rust execution client scaffold inspired by [ETH2030](https://eth2030.com/).

Status: experimental scaffold in active development.

## Workspace Layout

- `crates/reth2030` - CLI binary (`reth2030`)
- `crates/reth2030-core` - shared core types and config scaffolding
- `crates/reth2030-types` - execution-layer primitive types (tx, block, receipt)
- `code/` - local external reference code (ignored by git except `code/README.md`)

## Quick Start

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo run -p reth2030 -- --help
cargo run -p reth2030 -- --chain sepolia
```

## Notes

This repository is intentionally starting with minimal interfaces so protocol
components can be added incrementally with clear milestones.

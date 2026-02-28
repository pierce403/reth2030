# Contributing to reth2030

Thanks for contributing.

## Prerequisites

- Rust stable toolchain
- `cargo`
- Optional: `pre-commit` for local hooks

## Setup

```bash
git clone git@github.com:pierce403/reth2030.git
cd reth2030
cargo check --workspace
```

Optional pre-commit hooks:

```bash
pip install pre-commit
pre-commit install
```

## Development Workflow

1. Create a focused branch from `main`.
2. Make small, reviewable commits.
3. Run the full local checks before pushing:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Project Conventions

- Keep crate responsibilities narrow and explicit.
- Prefer deterministic tests over flaky integration setups.
- Add or update `AGENTS.md` when important implementation knowledge is learned.
- Keep external reference clones only under `code/`.

## Pull Requests

- Explain intent, behavior changes, and testing done.
- Link TODO phase/checkboxes when applicable.
- Keep PR scope minimal to simplify review.

//! Core interfaces and execution/state scaffolding for `reth2030`.
//!
//! ## Public API
//! - `Chain`: chain selector used by runtime and config defaults.
//! - `NodeConfig`: node runtime configuration values.
//! - `ExecutionEngine`: trait boundary for block execution backends.
//! - `SimpleExecutionEngine`: deterministic scaffold execution backend.
//! - `ExecutionError`: error type for execution pipeline failures.
//! - `TxExecutionResult`: per-transaction execution outcome.
//! - `BlockExecutionResult`: aggregate per-block execution outcome.
//! - `StateStore`: trait boundary for account/storage state backends.
//! - `InMemoryState`: deterministic in-memory `StateStore` implementation.
//! - `StateError`: error type for state transition failures.
//! - `Account`: account model used by `StateStore`.
//! - `StorageKey`: account storage key primitive.
//! - `StorageValue`: account storage value primitive.

mod config;
mod execution;
mod state;

pub use config::{Chain, NodeConfig};
pub use execution::{
    BlockExecutionResult, ExecutionEngine, ExecutionError, SimpleExecutionEngine, TxExecutionResult,
};
pub use state::{Account, InMemoryState, StateError, StateStore, StorageKey, StorageValue};

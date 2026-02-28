//! Execution-engine abstractions for block processing.

use crate::{StateError, StateStore};
use reth2030_types::{Block, Hash32, Receipt, ValidationError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxExecutionResult {
    pub tx_index: usize,
    pub gas_used: u64,
    pub cumulative_gas_used: u64,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockExecutionResult {
    pub tx_results: Vec<TxExecutionResult>,
    pub receipts: Vec<Receipt>,
    pub total_gas_used: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    InvalidBlock(ValidationError),
    State(StateError),
    GasLimitExceeded {
        gas_limit: u64,
        attempted: u64,
        tx_index: usize,
    },
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::InvalidBlock(err) => write!(f, "invalid block: {}", err),
            ExecutionError::State(err) => write!(f, "state transition error: {}", err),
            ExecutionError::GasLimitExceeded {
                gas_limit,
                attempted,
                tx_index,
            } => write!(
                f,
                "block gas limit exceeded at tx index {}: attempted={}, gas_limit={}",
                tx_index, attempted, gas_limit
            ),
        }
    }
}

impl std::error::Error for ExecutionError {}

impl From<ValidationError> for ExecutionError {
    fn from(value: ValidationError) -> Self {
        Self::InvalidBlock(value)
    }
}

impl From<StateError> for ExecutionError {
    fn from(value: StateError) -> Self {
        Self::State(value)
    }
}

pub trait ExecutionEngine {
    fn execute_block<S: StateStore>(
        &self,
        state: &mut S,
        block: &Block,
    ) -> Result<BlockExecutionResult, ExecutionError>;
}

#[derive(Debug, Clone)]
pub struct SimpleExecutionEngine {
    pub base_gas_per_tx: u64,
}

impl Default for SimpleExecutionEngine {
    fn default() -> Self {
        Self {
            base_gas_per_tx: 21_000,
        }
    }
}

impl SimpleExecutionEngine {
    pub fn new(base_gas_per_tx: u64) -> Self {
        Self { base_gas_per_tx }
    }

    fn gas_for_transaction(&self) -> u64 {
        // TODO(fork-rules): replace this constant with fork-aware per-tx gas schedules.
        self.base_gas_per_tx
    }
}

impl ExecutionEngine for SimpleExecutionEngine {
    fn execute_block<S: StateStore>(
        &self,
        state: &mut S,
        block: &Block,
    ) -> Result<BlockExecutionResult, ExecutionError> {
        block.validate_basic()?;

        let mut cumulative_gas = 0_u64;
        let mut tx_results = Vec::with_capacity(block.transactions.len());
        let mut receipts = Vec::with_capacity(block.transactions.len());

        for (index, tx) in block.transactions.iter().enumerate() {
            let gas_used = self.gas_for_transaction();
            let attempted = cumulative_gas.saturating_add(gas_used);
            if attempted > block.header.gas_limit {
                return Err(ExecutionError::GasLimitExceeded {
                    gas_limit: block.header.gas_limit,
                    attempted,
                    tx_index: index,
                });
            }

            state.apply_transaction(tx)?;
            cumulative_gas = attempted;

            tx_results.push(TxExecutionResult {
                tx_index: index,
                gas_used,
                cumulative_gas_used: cumulative_gas,
                success: true,
            });

            receipts.push(Receipt {
                tx_hash: pseudo_hash(tx.from(), tx.nonce(), index),
                success: true,
                cumulative_gas_used: cumulative_gas,
                logs: Vec::new(),
            });
        }

        Ok(BlockExecutionResult {
            tx_results,
            receipts,
            total_gas_used: cumulative_gas,
        })
    }
}

fn pseudo_hash(from: [u8; 20], nonce: u64, index: usize) -> Hash32 {
    let mut out = [0_u8; 32];
    out[..20].copy_from_slice(&from);
    out[20..28].copy_from_slice(&nonce.to_be_bytes());
    out[28..32].copy_from_slice(&(index as u32).to_be_bytes());
    out
}

//! Execution-engine abstractions for block processing.

use crate::{StateError, StateStore};
use reth2030_types::{Block, Hash32, Receipt, Transaction, ValidationError};
use sha2::{Digest, Sha256};

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
    TxGasLimitTooLow {
        tx_gas_limit: u64,
        required: u64,
        tx_index: usize,
    },
    GasOverflow {
        cumulative_gas: u64,
        gas_used: u64,
        tx_index: usize,
    },
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
            ExecutionError::TxGasLimitTooLow {
                tx_gas_limit,
                required,
                tx_index,
            } => write!(
                f,
                "tx intrinsic gas exceeds tx gas limit at index {}: required={}, tx_gas_limit={}",
                tx_index, required, tx_gas_limit
            ),
            ExecutionError::GasOverflow {
                cumulative_gas,
                gas_used,
                tx_index,
            } => write!(
                f,
                "gas accounting overflow at tx index {}: cumulative_gas={}, gas_used={}",
                tx_index, cumulative_gas, gas_used
            ),
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
            if gas_used > tx.gas_limit() {
                return Err(ExecutionError::TxGasLimitTooLow {
                    tx_gas_limit: tx.gas_limit(),
                    required: gas_used,
                    tx_index: index,
                });
            }

            let attempted =
                cumulative_gas
                    .checked_add(gas_used)
                    .ok_or(ExecutionError::GasOverflow {
                        cumulative_gas,
                        gas_used,
                        tx_index: index,
                    })?;
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
                tx_hash: pseudo_hash(tx),
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

fn pseudo_hash(tx: &Transaction) -> Hash32 {
    let mut hasher = Sha256::new();
    hasher.update(b"reth2030:tx-hash:v1");

    match tx {
        Transaction::Legacy(inner) => {
            hasher.update([0_u8]);
            hash_common_fields(
                &mut hasher,
                inner.nonce,
                inner.from,
                inner.to,
                inner.gas_limit,
            );
            hasher.update(inner.gas_price.to_be_bytes());
            hasher.update(inner.value.to_be_bytes());
            hash_payload(&mut hasher, &inner.data);
        }
        Transaction::Eip1559(inner) => {
            hasher.update([1_u8]);
            hash_common_fields(
                &mut hasher,
                inner.nonce,
                inner.from,
                inner.to,
                inner.gas_limit,
            );
            hasher.update(inner.max_fee_per_gas.to_be_bytes());
            hasher.update(inner.max_priority_fee_per_gas.to_be_bytes());
            hasher.update(inner.value.to_be_bytes());
            hash_payload(&mut hasher, &inner.data);
        }
        Transaction::Blob(inner) => {
            hasher.update([2_u8]);
            hash_common_fields(
                &mut hasher,
                inner.nonce,
                inner.from,
                inner.to,
                inner.gas_limit,
            );
            hasher.update(inner.max_fee_per_gas.to_be_bytes());
            hasher.update(inner.max_priority_fee_per_gas.to_be_bytes());
            hasher.update(inner.max_fee_per_blob_gas.to_be_bytes());
            hasher.update(inner.value.to_be_bytes());
            hash_payload(&mut hasher, &inner.data);
            hasher.update((inner.blob_versioned_hashes.len() as u64).to_be_bytes());
            for blob_hash in &inner.blob_versioned_hashes {
                hasher.update(blob_hash);
            }
        }
    }

    let digest = hasher.finalize();
    let mut out = [0_u8; 32];
    out.copy_from_slice(&digest);
    out
}

fn hash_common_fields(
    hasher: &mut Sha256,
    nonce: u64,
    from: [u8; 20],
    to: Option<[u8; 20]>,
    gas_limit: u64,
) {
    hasher.update(nonce.to_be_bytes());
    hasher.update(from);
    match to {
        Some(address) => {
            hasher.update([1_u8]);
            hasher.update(address);
        }
        None => hasher.update([0_u8]),
    }
    hasher.update(gas_limit.to_be_bytes());
}

fn hash_payload(hasher: &mut Sha256, payload: &[u8]) {
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
}

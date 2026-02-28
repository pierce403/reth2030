use crate::peer::{PeerId, PeerInfo, PeerManager, PeerManagerError};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderRef {
    pub number: u64,
    pub hash: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockBodyRef {
    pub tx_count: usize,
}

pub trait SyncSource {
    fn fetch_headers(&self, start: u64, limit: usize) -> Vec<HeaderRef>;
    fn fetch_body(&self, header: &HeaderRef) -> BlockBodyRef;
}

pub trait ExecutionSink {
    fn execute_synced_block(
        &mut self,
        header: &HeaderRef,
        body: &BlockBodyRef,
    ) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncStepReport {
    pub header_number: u64,
    pub tx_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncReport {
    pub steps: Vec<SyncStepReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    HeaderBatchTooLarge { limit: usize, received: usize },
    InvalidHeaderSequence { expected: u64, got: u64 },
    HeaderSequenceOverflow { last_header: u64 },
    ExecutionFailed { header_number: u64, reason: String },
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::HeaderBatchTooLarge { limit, received } => write!(
                f,
                "sync source returned {} headers, exceeding limit {}",
                received, limit
            ),
            SyncError::InvalidHeaderSequence { expected, got } => write!(
                f,
                "sync source returned out-of-sequence header: expected {}, got {}",
                expected, got
            ),
            SyncError::HeaderSequenceOverflow { last_header } => write!(
                f,
                "sync header sequence overflowed after header {}",
                last_header
            ),
            SyncError::ExecutionFailed {
                header_number,
                reason,
            } => write!(
                f,
                "execution failed for header {}: {}",
                header_number, reason
            ),
        }
    }
}

impl std::error::Error for SyncError {}

#[derive(Debug, Clone)]
pub struct SyncOrchestrator {
    pub peer_manager: PeerManager,
}

impl SyncOrchestrator {
    pub fn new(max_peers: usize) -> Self {
        Self {
            peer_manager: PeerManager::new(max_peers),
        }
    }

    pub fn connect_peer(&mut self, peer: PeerInfo) -> Result<(), PeerManagerError> {
        self.peer_manager.connect(peer)
    }

    pub fn disconnect_peer(&mut self, peer_id: &PeerId) -> bool {
        self.peer_manager.disconnect(peer_id)
    }

    pub fn run_once<S: SyncSource, E: ExecutionSink>(
        &mut self,
        source: &S,
        execution: &mut E,
        start: u64,
        limit: usize,
    ) -> Result<SyncReport, SyncError> {
        if limit == 0 {
            return Ok(SyncReport { steps: Vec::new() });
        }

        let headers = source.fetch_headers(start, limit);
        validate_header_batch(start, limit, &headers)?;
        let mut steps = Vec::with_capacity(headers.len());

        for header in headers {
            let body = source.fetch_body(&header);
            execution
                .execute_synced_block(&header, &body)
                .map_err(|reason| SyncError::ExecutionFailed {
                    header_number: header.number,
                    reason,
                })?;

            steps.push(SyncStepReport {
                header_number: header.number,
                tx_count: body.tx_count,
            });
        }

        Ok(SyncReport { steps })
    }
}

fn validate_header_batch(start: u64, limit: usize, headers: &[HeaderRef]) -> Result<(), SyncError> {
    if headers.len() > limit {
        return Err(SyncError::HeaderBatchTooLarge {
            limit,
            received: headers.len(),
        });
    }

    let mut expected = start;
    for (index, header) in headers.iter().enumerate() {
        if header.number != expected {
            return Err(SyncError::InvalidHeaderSequence {
                expected,
                got: header.number,
            });
        }

        if index < headers.len().saturating_sub(1) {
            expected = expected
                .checked_add(1)
                .ok_or(SyncError::HeaderSequenceOverflow {
                    last_header: header.number,
                })?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct MockSyncSource {
    headers: Vec<HeaderRef>,
    tx_counts: BTreeMap<u64, usize>,
}

impl MockSyncSource {
    pub fn from_header_numbers(numbers: &[u64]) -> Self {
        let mut tx_counts = BTreeMap::new();
        let headers = numbers
            .iter()
            .copied()
            .map(|number| {
                tx_counts.insert(number, (number % 3) as usize + 1);
                HeaderRef {
                    number,
                    hash: deterministic_hash(number),
                }
            })
            .collect();

        Self { headers, tx_counts }
    }

    pub fn with_tx_counts(entries: &[(u64, usize)]) -> Self {
        let mut tx_counts = BTreeMap::new();
        let headers = entries
            .iter()
            .map(|(number, tx_count)| {
                tx_counts.insert(*number, *tx_count);
                HeaderRef {
                    number: *number,
                    hash: deterministic_hash(*number),
                }
            })
            .collect();

        Self { headers, tx_counts }
    }
}

impl SyncSource for MockSyncSource {
    fn fetch_headers(&self, start: u64, limit: usize) -> Vec<HeaderRef> {
        self.headers
            .iter()
            .filter(|header| header.number >= start)
            .take(limit)
            .cloned()
            .collect()
    }

    fn fetch_body(&self, header: &HeaderRef) -> BlockBodyRef {
        let tx_count = self.tx_counts.get(&header.number).copied().unwrap_or(0);
        BlockBodyRef { tx_count }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RecordingExecutionSink {
    executed: Vec<(u64, usize)>,
    fail_on_header: Option<u64>,
}

impl RecordingExecutionSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_failure(header_number: u64) -> Self {
        Self {
            executed: Vec::new(),
            fail_on_header: Some(header_number),
        }
    }

    pub fn executed(&self) -> &[(u64, usize)] {
        self.executed.as_slice()
    }
}

impl ExecutionSink for RecordingExecutionSink {
    fn execute_synced_block(
        &mut self,
        header: &HeaderRef,
        body: &BlockBodyRef,
    ) -> Result<(), String> {
        if self.fail_on_header == Some(header.number) {
            return Err("forced failure".to_string());
        }

        self.executed.push((header.number, body.tx_count));
        Ok(())
    }
}

fn deterministic_hash(number: u64) -> [u8; 32] {
    let mut hash = [0_u8; 32];
    hash[..8].copy_from_slice(&number.to_be_bytes());
    hash[8..16].copy_from_slice(&number.wrapping_mul(31).to_be_bytes());
    hash[16..24].copy_from_slice(&number.wrapping_mul(131).to_be_bytes());
    hash[24..32].copy_from_slice(&number.wrapping_mul(1313).to_be_bytes());
    hash
}

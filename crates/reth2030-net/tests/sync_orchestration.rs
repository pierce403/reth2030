use reth2030_net::{
    BlockBodyRef, HeaderRef, MockSyncSource, PeerEvent, PeerInfo, RecordingExecutionSink,
    SyncError, SyncOrchestrator, SyncSource,
};
use std::collections::BTreeMap;

fn peer_id(byte: u8) -> [u8; 16] {
    [byte; 16]
}

#[derive(Debug, Clone)]
struct FixedHeaderSource {
    headers: Vec<HeaderRef>,
    tx_counts: BTreeMap<u64, usize>,
    ignore_limit: bool,
}

impl FixedHeaderSource {
    fn with_tx_counts(entries: &[(u64, usize)]) -> Self {
        let mut tx_counts = BTreeMap::new();
        let headers = entries
            .iter()
            .map(|(number, tx_count)| {
                tx_counts.insert(*number, *tx_count);
                HeaderRef {
                    number: *number,
                    hash: test_hash(*number),
                }
            })
            .collect();
        Self {
            headers,
            tx_counts,
            ignore_limit: false,
        }
    }

    fn ignoring_limit(mut self) -> Self {
        self.ignore_limit = true;
        self
    }
}

impl SyncSource for FixedHeaderSource {
    fn fetch_headers(&self, start: u64, limit: usize) -> Vec<HeaderRef> {
        let filtered = self.headers.iter().filter(|header| header.number >= start);
        if self.ignore_limit {
            filtered.cloned().collect()
        } else {
            filtered.take(limit).cloned().collect()
        }
    }

    fn fetch_body(&self, header: &HeaderRef) -> BlockBodyRef {
        let tx_count = self.tx_counts.get(&header.number).copied().unwrap_or(0);
        BlockBodyRef { tx_count }
    }
}

fn test_hash(number: u64) -> [u8; 32] {
    let mut hash = [0_u8; 32];
    hash[..8].copy_from_slice(&number.to_be_bytes());
    hash
}

#[test]
fn peer_lifecycle_events_are_observable() {
    let mut orchestrator = SyncOrchestrator::new(1);

    orchestrator
        .connect_peer(PeerInfo::new(peer_id(0x01), "127.0.0.1:30303"))
        .expect("first peer");

    let rejected = orchestrator.connect_peer(PeerInfo::new(peer_id(0x02), "127.0.0.1:30304"));
    assert!(rejected.is_err());

    orchestrator.disconnect_peer(&peer_id(0x01));

    assert_eq!(
        orchestrator.peer_manager.events(),
        &[
            PeerEvent::Connected(peer_id(0x01)),
            PeerEvent::RejectedMaxPeers(peer_id(0x02)),
            PeerEvent::Disconnected(peer_id(0x01)),
        ]
    );
}

#[test]
fn mocked_sync_loop_runs_without_external_network() {
    let mut orchestrator = SyncOrchestrator::new(4);
    let source = MockSyncSource::with_tx_counts(&[(1, 3), (2, 1), (3, 2)]);
    let mut sink = RecordingExecutionSink::new();

    let report = orchestrator
        .run_once(&source, &mut sink, 1, 3)
        .expect("sync report");

    assert_eq!(report.steps.len(), 3);
    assert_eq!(sink.executed(), &[(1, 3), (2, 1), (3, 2)]);
}

#[test]
fn sync_results_are_deterministic_across_runs() {
    let source = MockSyncSource::from_header_numbers(&[10, 11, 12]);
    let mut sink_a = RecordingExecutionSink::new();
    let mut sink_b = RecordingExecutionSink::new();

    let mut orchestrator_a = SyncOrchestrator::new(2);
    let mut orchestrator_b = SyncOrchestrator::new(2);

    let report_a = orchestrator_a
        .run_once(&source, &mut sink_a, 10, 3)
        .expect("first run");
    let report_b = orchestrator_b
        .run_once(&source, &mut sink_b, 10, 3)
        .expect("second run");

    assert_eq!(report_a, report_b);
    assert_eq!(sink_a.executed(), sink_b.executed());
}

#[test]
fn execution_failures_are_mapped_to_sync_error() {
    let source = MockSyncSource::with_tx_counts(&[(7, 1), (8, 1)]);
    let mut sink = RecordingExecutionSink::with_failure(8);
    let mut orchestrator = SyncOrchestrator::new(1);

    let err = orchestrator
        .run_once(&source, &mut sink, 7, 2)
        .expect_err("must fail on header 8");

    assert_eq!(
        err,
        SyncError::ExecutionFailed {
            header_number: 8,
            reason: "forced failure".to_string(),
        }
    );
    assert_eq!(sink.executed(), &[(7, 1)]);
}

#[test]
fn sync_rejects_sources_that_exceed_requested_limit() {
    let source = FixedHeaderSource::with_tx_counts(&[(1, 2), (2, 3)]).ignoring_limit();
    let mut sink = RecordingExecutionSink::new();
    let mut orchestrator = SyncOrchestrator::new(1);

    let err = orchestrator
        .run_once(&source, &mut sink, 1, 1)
        .expect_err("source must not return more headers than requested");

    assert_eq!(
        err,
        SyncError::HeaderBatchTooLarge {
            limit: 1,
            received: 2,
        }
    );
    assert!(sink.executed().is_empty());
}

#[test]
fn sync_rejects_sequence_when_start_header_is_missing() {
    let source = FixedHeaderSource::with_tx_counts(&[(5, 1)]);
    let mut sink = RecordingExecutionSink::new();
    let mut orchestrator = SyncOrchestrator::new(1);

    let err = orchestrator
        .run_once(&source, &mut sink, 4, 1)
        .expect_err("first header should match start");

    assert_eq!(
        err,
        SyncError::InvalidHeaderSequence {
            expected: 4,
            got: 5,
        }
    );
    assert!(sink.executed().is_empty());
}

#[test]
fn sync_rejects_gapped_header_sequence() {
    let source = FixedHeaderSource::with_tx_counts(&[(4, 1), (6, 1)]);
    let mut sink = RecordingExecutionSink::new();
    let mut orchestrator = SyncOrchestrator::new(1);

    let err = orchestrator
        .run_once(&source, &mut sink, 4, 2)
        .expect_err("gap in sequence must be rejected");

    assert_eq!(
        err,
        SyncError::InvalidHeaderSequence {
            expected: 5,
            got: 6,
        }
    );
    assert!(sink.executed().is_empty());
}

#[test]
fn sync_rejects_duplicate_header_numbers() {
    let source = FixedHeaderSource::with_tx_counts(&[(4, 1), (4, 2)]);
    let mut sink = RecordingExecutionSink::new();
    let mut orchestrator = SyncOrchestrator::new(1);

    let err = orchestrator
        .run_once(&source, &mut sink, 4, 2)
        .expect_err("duplicate header should be rejected");

    assert_eq!(
        err,
        SyncError::InvalidHeaderSequence {
            expected: 5,
            got: 4,
        }
    );
    assert!(sink.executed().is_empty());
}

#[test]
fn sync_detects_header_sequence_overflow() {
    let source = FixedHeaderSource::with_tx_counts(&[(u64::MAX, 1), (u64::MAX, 2)]);
    let mut sink = RecordingExecutionSink::new();
    let mut orchestrator = SyncOrchestrator::new(1);

    let err = orchestrator
        .run_once(&source, &mut sink, u64::MAX, 2)
        .expect_err("sequence overflow must be rejected");

    assert_eq!(
        err,
        SyncError::HeaderSequenceOverflow {
            last_header: u64::MAX,
        }
    );
    assert!(sink.executed().is_empty());
}

#[test]
fn sync_with_zero_limit_is_a_noop() {
    let source = MockSyncSource::with_tx_counts(&[(1, 1), (2, 2)]);
    let mut sink = RecordingExecutionSink::new();
    let mut orchestrator = SyncOrchestrator::new(1);

    let report = orchestrator
        .run_once(&source, &mut sink, 1, 0)
        .expect("zero limit should return an empty batch");

    assert!(report.steps.is_empty());
    assert!(sink.executed().is_empty());
}

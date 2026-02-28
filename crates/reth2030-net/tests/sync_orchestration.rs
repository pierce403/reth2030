use reth2030_net::{
    MockSyncSource, PeerEvent, PeerInfo, RecordingExecutionSink, SyncError, SyncOrchestrator,
};

fn peer_id(byte: u8) -> [u8; 16] {
    [byte; 16]
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
}

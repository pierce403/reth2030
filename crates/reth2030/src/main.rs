use clap::{Parser, ValueEnum};
use reth2030_core::{Chain, NodeConfig};
use reth2030_net::{
    MockSyncSource, PeerInfo, PeerManagerError, RecordingExecutionSink, SyncError, SyncOrchestrator,
};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliChain {
    Mainnet,
    Sepolia,
    Holesky,
}

impl From<CliChain> for Chain {
    fn from(value: CliChain) -> Self {
        match value {
            CliChain::Mainnet => Chain::Mainnet,
            CliChain::Sepolia => Chain::Sepolia,
            CliChain::Holesky => Chain::Holesky,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "reth2030")]
#[command(about = "Rust execution-client scaffold inspired by ETH2030")]
struct Cli {
    #[arg(long, value_enum, default_value = "mainnet")]
    chain: CliChain,

    #[arg(long, default_value = "~/.reth2030")]
    datadir: PathBuf,

    #[arg(long, default_value_t = 50)]
    maxpeers: usize,

    #[arg(long, default_value_t = false)]
    run_mock_sync: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeState {
    Initialized,
    Running,
    Stopped,
}

impl std::fmt::Display for RuntimeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeState::Initialized => write!(f, "initialized"),
            RuntimeState::Running => write!(f, "running"),
            RuntimeState::Stopped => write!(f, "stopped"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NodeRuntimeError {
    InvalidLifecycleState {
        action: &'static str,
        state: RuntimeState,
    },
    PeerManager(PeerManagerError),
    Sync(SyncError),
}

impl std::fmt::Display for NodeRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeRuntimeError::InvalidLifecycleState { action, state } => {
                write!(f, "cannot {} while runtime is {}", action, state)
            }
            NodeRuntimeError::PeerManager(err) => write!(f, "peer manager error: {}", err),
            NodeRuntimeError::Sync(err) => write!(f, "sync error: {}", err),
        }
    }
}

impl std::error::Error for NodeRuntimeError {}

struct NodeRuntime {
    config: NodeConfig,
    sync: SyncOrchestrator,
    lifecycle: RuntimeState,
}

impl NodeRuntime {
    fn new(config: NodeConfig) -> Self {
        let sync = SyncOrchestrator::new(config.max_peers);
        Self {
            config,
            sync,
            lifecycle: RuntimeState::Initialized,
        }
    }

    fn start(&mut self) -> Result<(), NodeRuntimeError> {
        if self.lifecycle != RuntimeState::Initialized {
            return Err(NodeRuntimeError::InvalidLifecycleState {
                action: "start",
                state: self.lifecycle,
            });
        }

        println!("reth2030 scaffold");
        println!("chain: {}", self.config.chain);
        println!("datadir: {}", self.config.datadir.display());
        println!("http_port: {}", self.config.http_port);
        println!("authrpc_port: {}", self.config.authrpc_port);
        println!("p2p_port: {}", self.config.p2p_port);
        println!("max_peers: {}", self.config.max_peers);

        self.lifecycle = RuntimeState::Running;
        Ok(())
    }

    fn execute(&mut self, run_mock_sync: bool) -> Result<(), NodeRuntimeError> {
        self.start()?;

        let run_result = if run_mock_sync {
            self.run_mock_sync_once()
        } else {
            Ok(())
        };

        let shutdown_result = self.shutdown();
        run_result?;
        shutdown_result
    }

    fn run_mock_sync_once(&mut self) -> Result<(), NodeRuntimeError> {
        if self.lifecycle != RuntimeState::Running {
            return Err(NodeRuntimeError::InvalidLifecycleState {
                action: "run mock sync",
                state: self.lifecycle,
            });
        }

        self.sync
            .connect_peer(PeerInfo::new(mock_peer_id(1), "127.0.0.1:30303"))
            .map_err(NodeRuntimeError::PeerManager)?;

        let source = MockSyncSource::with_tx_counts(&[(1, 2), (2, 1), (3, 3)]);
        let mut sink = RecordingExecutionSink::new();

        let report = self
            .sync
            .run_once(&source, &mut sink, 1, 3)
            .map_err(NodeRuntimeError::Sync)?;

        println!("mock sync processed {} headers", report.steps.len());
        println!("executed blocks: {}", sink.executed().len());
        println!(
            "peer events observed: {}",
            self.sync.peer_manager.events().len()
        );
        println!(
            "peer log entries observed: {}",
            self.sync.peer_manager.lifecycle_logs().len()
        );
        let (connected_total, disconnected_total, rejected_total, active_peers) =
            self.sync.peer_manager.metrics_snapshot();
        println!(
            "peer metrics stub: connected_total={} disconnected_total={} rejected_max_peers_total={} active_peers={}",
            connected_total, disconnected_total, rejected_total, active_peers
        );
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), NodeRuntimeError> {
        if self.lifecycle != RuntimeState::Running {
            return Err(NodeRuntimeError::InvalidLifecycleState {
                action: "shutdown",
                state: self.lifecycle,
            });
        }

        let disconnected = self.disconnect_all_peers();
        println!("disconnected peers: {}", disconnected);
        println!("node shutdown complete");
        self.lifecycle = RuntimeState::Stopped;
        Ok(())
    }

    fn disconnect_all_peers(&mut self) -> usize {
        let peer_ids: Vec<_> = self
            .sync
            .peer_manager
            .connected_peers()
            .into_iter()
            .map(|peer| peer.id)
            .collect();

        for peer_id in &peer_ids {
            self.sync.disconnect_peer(peer_id);
        }

        peer_ids.len()
    }
}

fn mock_peer_id(seed: u8) -> [u8; 16] {
    [seed; 16]
}

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), NodeRuntimeError> {
    let chain = Chain::from(cli.chain);

    let mut config = NodeConfig::default_for(chain);
    config.datadir = cli.datadir;
    config.max_peers = cli.maxpeers;

    let mut runtime = NodeRuntime::new(config);
    runtime.execute(cli.run_mock_sync)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reth2030_net::PeerEvent;

    fn runtime_with_max_peers(max_peers: usize) -> NodeRuntime {
        let mut config = NodeConfig::default_for(Chain::Mainnet);
        config.max_peers = max_peers;
        NodeRuntime::new(config)
    }

    fn expected_peer_log(action: &str, seed: u8, active_peers: usize) -> String {
        let peer_id = format!("{seed:02x}").repeat(16);
        format!("peer.{action} peer_id={peer_id} active_peers={active_peers}")
    }

    #[test]
    fn mock_sync_loop_runs_without_error() {
        let mut runtime = runtime_with_max_peers(1);
        runtime.start().expect("runtime should start");

        runtime
            .run_mock_sync_once()
            .expect("mock sync should complete");

        assert_eq!(runtime.lifecycle, RuntimeState::Running);
        assert_eq!(runtime.sync.peer_manager.peer_count(), 1);
        assert_eq!(
            runtime.sync.peer_manager.events(),
            &[PeerEvent::Connected(mock_peer_id(1))]
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            &[expected_peer_log("connected", 1, 1)]
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (1, 0, 0, 1));

        runtime.shutdown().expect("runtime should shut down");
        assert_eq!(runtime.lifecycle, RuntimeState::Stopped);
        assert_eq!(runtime.sync.peer_manager.peer_count(), 0);
        assert_eq!(
            runtime.sync.peer_manager.events(),
            &[
                PeerEvent::Connected(mock_peer_id(1)),
                PeerEvent::Disconnected(mock_peer_id(1)),
            ]
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            &[
                expected_peer_log("connected", 1, 1),
                expected_peer_log("disconnected", 1, 0),
            ]
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (1, 1, 0, 0));
    }

    #[test]
    fn mock_sync_loop_fails_closed_when_no_peer_slots_are_available() {
        let mut runtime = runtime_with_max_peers(0);
        runtime.start().expect("runtime should start");

        let err = runtime
            .run_mock_sync_once()
            .expect_err("mock sync should fail when max peers is zero");

        assert_eq!(
            err,
            NodeRuntimeError::PeerManager(PeerManagerError::MaxPeersReached { max_peers: 0 })
        );
        assert_eq!(runtime.lifecycle, RuntimeState::Running);
        assert_eq!(runtime.sync.peer_manager.peer_count(), 0);
        assert_eq!(
            runtime.sync.peer_manager.events(),
            &[PeerEvent::RejectedMaxPeers(mock_peer_id(1))]
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            &[expected_peer_log("rejected_max_peers", 1, 0)]
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (0, 0, 1, 0));

        runtime.shutdown().expect("runtime should shut down");
        assert_eq!(runtime.lifecycle, RuntimeState::Stopped);
    }

    #[test]
    fn mock_sync_loop_can_run_repeatedly_without_panicking() {
        let mut runtime = runtime_with_max_peers(1);
        runtime.start().expect("runtime should start");

        runtime
            .run_mock_sync_once()
            .expect("first mock sync run should succeed");
        runtime
            .run_mock_sync_once()
            .expect("second mock sync run should also succeed");

        assert_eq!(runtime.lifecycle, RuntimeState::Running);
        assert_eq!(runtime.sync.peer_manager.peer_count(), 1);
        assert_eq!(
            runtime.sync.peer_manager.events(),
            &[
                PeerEvent::Connected(mock_peer_id(1)),
                PeerEvent::Connected(mock_peer_id(1)),
            ]
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            &[
                expected_peer_log("connected", 1, 1),
                expected_peer_log("connected", 1, 1),
            ]
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (2, 0, 0, 1));

        runtime.shutdown().expect("runtime should shut down");
        assert_eq!(runtime.sync.peer_manager.peer_count(), 0);
        assert_eq!(
            runtime.sync.peer_manager.events(),
            &[
                PeerEvent::Connected(mock_peer_id(1)),
                PeerEvent::Connected(mock_peer_id(1)),
                PeerEvent::Disconnected(mock_peer_id(1)),
            ]
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            &[
                expected_peer_log("connected", 1, 1),
                expected_peer_log("connected", 1, 1),
                expected_peer_log("disconnected", 1, 0),
            ]
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (2, 1, 0, 0));
    }

    #[test]
    fn mock_sync_loop_reconnects_in_place_when_slot_is_occupied_by_the_same_peer() {
        let mut runtime = runtime_with_max_peers(1);
        runtime.start().expect("runtime should start");

        runtime
            .sync
            .connect_peer(PeerInfo::new(mock_peer_id(1), "127.0.0.1:30333"))
            .expect("pre-connected mock peer should occupy the only slot");
        let initial_session_id = runtime
            .sync
            .peer_manager
            .session(&mock_peer_id(1))
            .expect("pre-connected session should exist")
            .session_id;
        assert_eq!(runtime.sync.peer_manager.peer_count(), 1);
        assert_eq!(
            runtime.sync.peer_manager.connected_peers(),
            vec![PeerInfo::new(mock_peer_id(1), "127.0.0.1:30333")]
        );

        runtime
            .run_mock_sync_once()
            .expect("mock sync should reconnect in place for the same peer id");

        let session_after_sync = runtime
            .sync
            .peer_manager
            .session(&mock_peer_id(1))
            .expect("session should remain present after reconnect");
        assert_eq!(session_after_sync.session_id, initial_session_id + 1);
        assert_eq!(session_after_sync.peer.address, "127.0.0.1:30303");
        assert_eq!(runtime.lifecycle, RuntimeState::Running);
        assert_eq!(runtime.sync.peer_manager.peer_count(), 1);
        assert_eq!(
            runtime.sync.peer_manager.connected_peers(),
            vec![PeerInfo::new(mock_peer_id(1), "127.0.0.1:30303")]
        );
        assert_eq!(
            runtime.sync.peer_manager.events(),
            &[
                PeerEvent::Connected(mock_peer_id(1)),
                PeerEvent::Connected(mock_peer_id(1)),
            ]
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            &[
                expected_peer_log("connected", 1, 1),
                expected_peer_log("connected", 1, 1),
            ]
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (2, 0, 0, 1));

        runtime.shutdown().expect("runtime should shut down");
        assert_eq!(runtime.lifecycle, RuntimeState::Stopped);
        assert_eq!(runtime.sync.peer_manager.peer_count(), 0);
        assert_eq!(
            runtime.sync.peer_manager.events(),
            &[
                PeerEvent::Connected(mock_peer_id(1)),
                PeerEvent::Connected(mock_peer_id(1)),
                PeerEvent::Disconnected(mock_peer_id(1)),
            ]
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            &[
                expected_peer_log("connected", 1, 1),
                expected_peer_log("connected", 1, 1),
                expected_peer_log("disconnected", 1, 0),
            ]
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (2, 1, 0, 0));
    }

    #[test]
    fn mock_sync_loop_retries_fail_closed_when_peer_slot_is_taken_and_recovers_when_freed() {
        let mut runtime = runtime_with_max_peers(1);
        runtime.start().expect("runtime should start");

        runtime
            .sync
            .connect_peer(PeerInfo::new(mock_peer_id(2), "127.0.0.1:30304"))
            .expect("saturating peer should connect");
        assert_eq!(runtime.sync.peer_manager.peer_count(), 1);

        for _ in 0..2 {
            let err = runtime
                .run_mock_sync_once()
                .expect_err("mock sync should fail while another peer occupies the only slot");
            assert_eq!(
                err,
                NodeRuntimeError::PeerManager(PeerManagerError::MaxPeersReached { max_peers: 1 })
            );
        }

        assert_eq!(runtime.lifecycle, RuntimeState::Running);
        assert_eq!(runtime.sync.peer_manager.peer_count(), 1);
        assert_eq!(
            runtime.sync.peer_manager.connected_peers(),
            vec![PeerInfo::new(mock_peer_id(2), "127.0.0.1:30304")]
        );
        assert_eq!(
            runtime.sync.peer_manager.events(),
            &[
                PeerEvent::Connected(mock_peer_id(2)),
                PeerEvent::RejectedMaxPeers(mock_peer_id(1)),
                PeerEvent::RejectedMaxPeers(mock_peer_id(1)),
            ]
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            &[
                expected_peer_log("connected", 2, 1),
                expected_peer_log("rejected_max_peers", 1, 1),
                expected_peer_log("rejected_max_peers", 1, 1),
            ]
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (1, 0, 2, 1));

        assert!(
            runtime.sync.disconnect_peer(&mock_peer_id(2)),
            "pre-connected peer should disconnect to free a slot"
        );

        runtime
            .run_mock_sync_once()
            .expect("mock sync should succeed once a peer slot is freed");
        assert_eq!(runtime.sync.peer_manager.peer_count(), 1);
        assert_eq!(
            runtime.sync.peer_manager.connected_peers(),
            vec![PeerInfo::new(mock_peer_id(1), "127.0.0.1:30303")]
        );
        assert_eq!(
            runtime.sync.peer_manager.events(),
            &[
                PeerEvent::Connected(mock_peer_id(2)),
                PeerEvent::RejectedMaxPeers(mock_peer_id(1)),
                PeerEvent::RejectedMaxPeers(mock_peer_id(1)),
                PeerEvent::Disconnected(mock_peer_id(2)),
                PeerEvent::Connected(mock_peer_id(1)),
            ]
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            &[
                expected_peer_log("connected", 2, 1),
                expected_peer_log("rejected_max_peers", 1, 1),
                expected_peer_log("rejected_max_peers", 1, 1),
                expected_peer_log("disconnected", 2, 0),
                expected_peer_log("connected", 1, 1),
            ]
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (2, 1, 2, 1));

        runtime.shutdown().expect("runtime should shut down");
        assert_eq!(runtime.lifecycle, RuntimeState::Stopped);
        assert_eq!(runtime.sync.peer_manager.peer_count(), 0);
        assert_eq!(
            runtime.sync.peer_manager.events(),
            &[
                PeerEvent::Connected(mock_peer_id(2)),
                PeerEvent::RejectedMaxPeers(mock_peer_id(1)),
                PeerEvent::RejectedMaxPeers(mock_peer_id(1)),
                PeerEvent::Disconnected(mock_peer_id(2)),
                PeerEvent::Connected(mock_peer_id(1)),
                PeerEvent::Disconnected(mock_peer_id(1)),
            ]
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            &[
                expected_peer_log("connected", 2, 1),
                expected_peer_log("rejected_max_peers", 1, 1),
                expected_peer_log("rejected_max_peers", 1, 1),
                expected_peer_log("disconnected", 2, 0),
                expected_peer_log("connected", 1, 1),
                expected_peer_log("disconnected", 1, 0),
            ]
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (2, 2, 2, 0));
    }

    #[test]
    fn runtime_rejects_invalid_lifecycle_transitions() {
        let mut runtime = runtime_with_max_peers(1);

        let err = runtime
            .shutdown()
            .expect_err("shutdown should fail before start");
        assert_eq!(
            err,
            NodeRuntimeError::InvalidLifecycleState {
                action: "shutdown",
                state: RuntimeState::Initialized,
            }
        );

        let err = runtime
            .run_mock_sync_once()
            .expect_err("mock sync should fail before start");
        assert_eq!(
            err,
            NodeRuntimeError::InvalidLifecycleState {
                action: "run mock sync",
                state: RuntimeState::Initialized,
            }
        );

        runtime.start().expect("runtime should start");

        let err = runtime.start().expect_err("double-start should fail");
        assert_eq!(
            err,
            NodeRuntimeError::InvalidLifecycleState {
                action: "start",
                state: RuntimeState::Running,
            }
        );

        runtime.shutdown().expect("runtime should shut down");

        let err = runtime
            .start()
            .expect_err("restarting after shutdown should fail closed");
        assert_eq!(
            err,
            NodeRuntimeError::InvalidLifecycleState {
                action: "start",
                state: RuntimeState::Stopped,
            }
        );

        let err = runtime
            .run_mock_sync_once()
            .expect_err("mock sync should fail after shutdown");
        assert_eq!(
            err,
            NodeRuntimeError::InvalidLifecycleState {
                action: "run mock sync",
                state: RuntimeState::Stopped,
            }
        );

        let err = runtime
            .shutdown()
            .expect_err("shutdown should fail after shutdown");
        assert_eq!(
            err,
            NodeRuntimeError::InvalidLifecycleState {
                action: "shutdown",
                state: RuntimeState::Stopped,
            }
        );
    }

    #[test]
    fn shutdown_disconnects_all_connected_peers() {
        let mut runtime = runtime_with_max_peers(2);
        runtime.start().expect("runtime should start");

        runtime
            .sync
            .connect_peer(PeerInfo::new(mock_peer_id(1), "127.0.0.1:30303"))
            .expect("peer 1 should connect");
        runtime
            .sync
            .connect_peer(PeerInfo::new(mock_peer_id(2), "127.0.0.1:30304"))
            .expect("peer 2 should connect");
        assert_eq!(runtime.sync.peer_manager.peer_count(), 2);

        runtime.shutdown().expect("runtime should shut down");
        assert_eq!(runtime.lifecycle, RuntimeState::Stopped);
        assert_eq!(runtime.sync.peer_manager.peer_count(), 0);
        assert_eq!(
            runtime.sync.peer_manager.events(),
            &[
                PeerEvent::Connected(mock_peer_id(1)),
                PeerEvent::Connected(mock_peer_id(2)),
                PeerEvent::Disconnected(mock_peer_id(1)),
                PeerEvent::Disconnected(mock_peer_id(2)),
            ]
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            &[
                expected_peer_log("connected", 1, 1),
                expected_peer_log("connected", 2, 2),
                expected_peer_log("disconnected", 1, 1),
                expected_peer_log("disconnected", 2, 0),
            ]
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (2, 2, 0, 0));
    }

    #[test]
    fn shutdown_disconnects_peers_in_stable_peer_id_order() {
        let mut runtime = runtime_with_max_peers(3);
        runtime.start().expect("runtime should start");

        runtime
            .sync
            .connect_peer(PeerInfo::new(mock_peer_id(3), "127.0.0.1:30305"))
            .expect("peer 3 should connect");
        runtime
            .sync
            .connect_peer(PeerInfo::new(mock_peer_id(1), "127.0.0.1:30303"))
            .expect("peer 1 should connect");
        runtime
            .sync
            .connect_peer(PeerInfo::new(mock_peer_id(2), "127.0.0.1:30304"))
            .expect("peer 2 should connect");
        assert_eq!(runtime.sync.peer_manager.peer_count(), 3);

        runtime.shutdown().expect("runtime should shut down");
        assert_eq!(runtime.lifecycle, RuntimeState::Stopped);
        assert_eq!(runtime.sync.peer_manager.peer_count(), 0);
        assert_eq!(
            runtime.sync.peer_manager.events(),
            &[
                PeerEvent::Connected(mock_peer_id(3)),
                PeerEvent::Connected(mock_peer_id(1)),
                PeerEvent::Connected(mock_peer_id(2)),
                PeerEvent::Disconnected(mock_peer_id(1)),
                PeerEvent::Disconnected(mock_peer_id(2)),
                PeerEvent::Disconnected(mock_peer_id(3)),
            ]
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            &[
                expected_peer_log("connected", 3, 1),
                expected_peer_log("connected", 1, 2),
                expected_peer_log("connected", 2, 3),
                expected_peer_log("disconnected", 1, 2),
                expected_peer_log("disconnected", 2, 1),
                expected_peer_log("disconnected", 3, 0),
            ]
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (3, 3, 0, 0));
    }

    #[test]
    fn runtime_execute_without_mock_sync_starts_and_stops() {
        let mut runtime = runtime_with_max_peers(1);

        runtime
            .execute(false)
            .expect("runtime execute without mock sync should succeed");

        assert_eq!(runtime.lifecycle, RuntimeState::Stopped);
        assert_eq!(runtime.sync.peer_manager.peer_count(), 0);
        assert!(
            runtime.sync.peer_manager.events().is_empty(),
            "no peer events should be emitted when mock sync is disabled"
        );
        assert!(
            runtime.sync.peer_manager.lifecycle_logs().is_empty(),
            "no peer lifecycle logs should be emitted when mock sync is disabled"
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (0, 0, 0, 0));
    }

    #[test]
    fn runtime_execute_with_mock_sync_success_stops_and_disconnects() {
        let mut runtime = runtime_with_max_peers(1);

        runtime
            .execute(true)
            .expect("runtime execute with mock sync should succeed");

        assert_eq!(runtime.lifecycle, RuntimeState::Stopped);
        assert_eq!(runtime.sync.peer_manager.peer_count(), 0);
        assert_eq!(
            runtime.sync.peer_manager.events(),
            &[
                PeerEvent::Connected(mock_peer_id(1)),
                PeerEvent::Disconnected(mock_peer_id(1)),
            ]
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            &[
                expected_peer_log("connected", 1, 1),
                expected_peer_log("disconnected", 1, 0),
            ]
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (1, 1, 0, 0));
    }

    #[test]
    fn runtime_execute_with_mock_sync_failure_still_shuts_down() {
        let mut runtime = runtime_with_max_peers(0);

        let err = runtime
            .execute(true)
            .expect_err("runtime execute should return mock sync failure");

        assert_eq!(
            err,
            NodeRuntimeError::PeerManager(PeerManagerError::MaxPeersReached { max_peers: 0 })
        );
        assert_eq!(
            runtime.lifecycle,
            RuntimeState::Stopped,
            "execute must always attempt shutdown after start"
        );
        assert_eq!(runtime.sync.peer_manager.peer_count(), 0);
        assert_eq!(
            runtime.sync.peer_manager.events(),
            &[PeerEvent::RejectedMaxPeers(mock_peer_id(1))]
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            &[expected_peer_log("rejected_max_peers", 1, 0)]
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (0, 0, 1, 0));
    }

    #[test]
    fn runtime_execute_with_mock_sync_failure_disconnects_preexisting_peers_on_shutdown() {
        let mut runtime = runtime_with_max_peers(1);

        runtime
            .sync
            .connect_peer(PeerInfo::new(mock_peer_id(2), "127.0.0.1:30304"))
            .expect("pre-existing peer should connect before execute");
        assert_eq!(runtime.sync.peer_manager.peer_count(), 1);
        assert_eq!(
            runtime.sync.peer_manager.connected_peers(),
            vec![PeerInfo::new(mock_peer_id(2), "127.0.0.1:30304")]
        );

        let err = runtime
            .execute(true)
            .expect_err("runtime execute should return mock sync max-peer failure");

        assert_eq!(
            err,
            NodeRuntimeError::PeerManager(PeerManagerError::MaxPeersReached { max_peers: 1 })
        );
        assert_eq!(
            runtime.lifecycle,
            RuntimeState::Stopped,
            "execute must still shut down after mock-sync failure with connected peers"
        );
        assert_eq!(runtime.sync.peer_manager.peer_count(), 0);
        assert_eq!(
            runtime.sync.peer_manager.connected_peers(),
            Vec::<PeerInfo>::new()
        );
        assert_eq!(
            runtime.sync.peer_manager.events(),
            &[
                PeerEvent::Connected(mock_peer_id(2)),
                PeerEvent::RejectedMaxPeers(mock_peer_id(1)),
                PeerEvent::Disconnected(mock_peer_id(2)),
            ]
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            &[
                expected_peer_log("connected", 2, 1),
                expected_peer_log("rejected_max_peers", 1, 1),
                expected_peer_log("disconnected", 2, 0),
            ]
        );
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (1, 1, 1, 0));
    }

    #[test]
    fn execute_fails_closed_when_called_while_running() {
        let mut runtime = runtime_with_max_peers(1);
        runtime.start().expect("runtime should start");

        let err = runtime
            .execute(false)
            .expect_err("execute should fail when runtime is already running");

        assert_eq!(
            err,
            NodeRuntimeError::InvalidLifecycleState {
                action: "start",
                state: RuntimeState::Running,
            }
        );
        assert_eq!(
            runtime.lifecycle,
            RuntimeState::Running,
            "failed execute must not mutate lifecycle when start fails"
        );
        assert_eq!(runtime.sync.peer_manager.peer_count(), 0);
        assert!(runtime.sync.peer_manager.events().is_empty());
        assert!(runtime.sync.peer_manager.lifecycle_logs().is_empty());
        assert_eq!(runtime.sync.peer_manager.metrics_snapshot(), (0, 0, 0, 0));

        runtime
            .shutdown()
            .expect("runtime should still shut down cleanly");
        assert_eq!(runtime.lifecycle, RuntimeState::Stopped);
    }

    #[test]
    fn execute_fails_closed_when_called_after_shutdown() {
        let mut runtime = runtime_with_max_peers(1);
        runtime
            .execute(true)
            .expect("first execute should start, run mock sync, and shut down");

        assert_eq!(runtime.lifecycle, RuntimeState::Stopped);
        let baseline_peer_count = runtime.sync.peer_manager.peer_count();
        let baseline_events = runtime.sync.peer_manager.events().to_vec();
        let baseline_logs = runtime.sync.peer_manager.lifecycle_logs().to_vec();
        let baseline_metrics = runtime.sync.peer_manager.metrics_snapshot();

        let err = runtime
            .execute(true)
            .expect_err("execute should fail closed after runtime is already stopped");

        assert_eq!(
            err,
            NodeRuntimeError::InvalidLifecycleState {
                action: "start",
                state: RuntimeState::Stopped,
            }
        );
        assert_eq!(
            runtime.lifecycle,
            RuntimeState::Stopped,
            "failed execute must not mutate lifecycle when start fails from stopped"
        );
        assert_eq!(runtime.sync.peer_manager.peer_count(), baseline_peer_count);
        assert_eq!(
            runtime.sync.peer_manager.events(),
            baseline_events.as_slice()
        );
        assert_eq!(
            runtime.sync.peer_manager.lifecycle_logs(),
            baseline_logs.as_slice()
        );
        assert_eq!(
            runtime.sync.peer_manager.metrics_snapshot(),
            baseline_metrics
        );
    }
}

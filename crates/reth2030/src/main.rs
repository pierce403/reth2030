use clap::{Parser, ValueEnum};
use reth2030_core::{Chain, NodeConfig};
use reth2030_net::{MockSyncSource, PeerInfo, RecordingExecutionSink, SyncOrchestrator};
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

struct NodeRuntime {
    config: NodeConfig,
    sync: SyncOrchestrator,
}

impl NodeRuntime {
    fn new(config: NodeConfig) -> Self {
        let sync = SyncOrchestrator::new(config.max_peers);
        Self { config, sync }
    }

    fn start(&self) {
        println!("reth2030 scaffold");
        println!("chain: {}", self.config.chain);
        println!("datadir: {}", self.config.datadir.display());
        println!("http_port: {}", self.config.http_port);
        println!("authrpc_port: {}", self.config.authrpc_port);
        println!("p2p_port: {}", self.config.p2p_port);
        println!("max_peers: {}", self.config.max_peers);
    }

    fn run_mock_sync_once(&mut self) -> Result<(), String> {
        self.sync
            .connect_peer(PeerInfo::new(mock_peer_id(1), "127.0.0.1:30303"))
            .map_err(|err| err.to_string())?;

        let source = MockSyncSource::with_tx_counts(&[(1, 2), (2, 1), (3, 3)]);
        let mut sink = RecordingExecutionSink::new();

        let report = self
            .sync
            .run_once(&source, &mut sink, 1, 3)
            .map_err(|err| err.to_string())?;

        println!("mock sync processed {} headers", report.steps.len());
        println!("executed blocks: {}", sink.executed().len());
        println!(
            "peer events observed: {}",
            self.sync.peer_manager.events().len()
        );
        Ok(())
    }

    fn shutdown(&self) {
        println!("node shutdown complete");
    }
}

fn mock_peer_id(seed: u8) -> [u8; 16] {
    [seed; 16]
}

fn main() {
    let cli = Cli::parse();
    let chain = Chain::from(cli.chain);

    let mut config = NodeConfig::default_for(chain);
    config.datadir = cli.datadir;
    config.max_peers = cli.maxpeers;

    let mut runtime = NodeRuntime::new(config);
    runtime.start();

    if cli.run_mock_sync {
        if let Err(err) = runtime.run_mock_sync_once() {
            eprintln!("mock sync failed: {}", err);
            std::process::exit(1);
        }
    }

    runtime.shutdown();
}

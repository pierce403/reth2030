use clap::{Parser, ValueEnum};
use reth2030_core::{Chain, NodeConfig};
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
}

fn main() {
    let cli = Cli::parse();
    let chain = Chain::from(cli.chain);

    let mut config = NodeConfig::default_for(chain);
    config.datadir = cli.datadir;

    println!("reth2030 scaffold");
    println!("chain: {}", config.chain);
    println!("datadir: {}", config.datadir.display());
    println!("http_port: {}", config.http_port);
    println!("authrpc_port: {}", config.authrpc_port);
    println!("p2p_port: {}", config.p2p_port);
}

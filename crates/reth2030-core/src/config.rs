use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chain {
    Mainnet,
    Sepolia,
    Holesky,
}

impl Chain {
    pub const fn as_str(self) -> &'static str {
        match self {
            Chain::Mainnet => "mainnet",
            Chain::Sepolia => "sepolia",
            Chain::Holesky => "holesky",
        }
    }
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeConfig {
    pub chain: Chain,
    pub datadir: PathBuf,
    pub http_port: u16,
    pub authrpc_port: u16,
    pub p2p_port: u16,
    pub max_peers: usize,
}

impl NodeConfig {
    pub fn default_for(chain: Chain) -> Self {
        Self {
            chain,
            datadir: default_datadir(),
            http_port: 8545,
            authrpc_port: 8551,
            p2p_port: 30303,
            max_peers: 50,
        }
    }
}

fn default_datadir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".reth2030")
    } else {
        PathBuf::from("~/.reth2030")
    }
}

#[cfg(test)]
mod tests {
    use super::{Chain, NodeConfig};

    #[test]
    fn default_for_uses_expected_ports() {
        let cfg = NodeConfig::default_for(Chain::Mainnet);
        assert_eq!(cfg.http_port, 8545);
        assert_eq!(cfg.authrpc_port, 8551);
        assert_eq!(cfg.p2p_port, 30303);
        assert_eq!(cfg.max_peers, 50);
    }

    #[test]
    fn default_for_preserves_chain() {
        let cfg = NodeConfig::default_for(Chain::Sepolia);
        assert_eq!(cfg.chain, Chain::Sepolia);
    }

    #[test]
    fn datadir_defaults_to_dot_reth2030() {
        let cfg = NodeConfig::default_for(Chain::Holesky);
        let rendered = cfg.datadir.display().to_string();
        assert!(rendered.ends_with(".reth2030"));
    }

    #[test]
    fn chain_display_matches_str() {
        assert_eq!(Chain::Mainnet.to_string(), "mainnet");
        assert_eq!(Chain::Sepolia.to_string(), "sepolia");
        assert_eq!(Chain::Holesky.to_string(), "holesky");
    }
}

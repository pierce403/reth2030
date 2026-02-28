use std::collections::BTreeMap;

pub type PeerId = [u8; 16];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerInfo {
    pub id: PeerId,
    pub address: String,
}

impl PeerInfo {
    pub fn new(id: PeerId, address: impl Into<String>) -> Self {
        Self {
            id,
            address: address.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerEvent {
    Connected(PeerId),
    Disconnected(PeerId),
    RejectedMaxPeers(PeerId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerManagerError {
    MaxPeersReached { max_peers: usize },
}

impl std::fmt::Display for PeerManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeerManagerError::MaxPeersReached { max_peers } => {
                write!(f, "maximum peers reached ({})", max_peers)
            }
        }
    }
}

impl std::error::Error for PeerManagerError {}

#[derive(Debug, Clone)]
pub struct PeerManager {
    max_peers: usize,
    peers: BTreeMap<PeerId, PeerInfo>,
    events: Vec<PeerEvent>,
}

impl PeerManager {
    pub fn new(max_peers: usize) -> Self {
        Self {
            max_peers,
            peers: BTreeMap::new(),
            events: Vec::new(),
        }
    }

    pub fn connect(&mut self, peer: PeerInfo) -> Result<(), PeerManagerError> {
        if self.peers.len() >= self.max_peers && !self.peers.contains_key(&peer.id) {
            self.events.push(PeerEvent::RejectedMaxPeers(peer.id));
            return Err(PeerManagerError::MaxPeersReached {
                max_peers: self.max_peers,
            });
        }

        let peer_id = peer.id;
        self.peers.insert(peer_id, peer);
        self.events.push(PeerEvent::Connected(peer_id));
        Ok(())
    }

    pub fn disconnect(&mut self, peer_id: &PeerId) -> bool {
        let removed = self.peers.remove(peer_id).is_some();
        if removed {
            self.events.push(PeerEvent::Disconnected(*peer_id));
        }
        removed
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    pub fn max_peers(&self) -> usize {
        self.max_peers
    }

    pub fn connected_peers(&self) -> Vec<PeerInfo> {
        self.peers.values().cloned().collect()
    }

    pub fn events(&self) -> &[PeerEvent] {
        self.events.as_slice()
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }
}

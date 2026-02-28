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
pub struct PeerSession {
    pub session_id: u64,
    pub peer: PeerInfo,
}

impl PeerSession {
    fn new(session_id: u64, peer: PeerInfo) -> Self {
        Self { session_id, peer }
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
    SessionIdOverflow,
}

impl std::fmt::Display for PeerManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeerManagerError::MaxPeersReached { max_peers } => {
                write!(f, "maximum peers reached ({})", max_peers)
            }
            PeerManagerError::SessionIdOverflow => {
                write!(f, "peer session identifier overflowed")
            }
        }
    }
}

impl std::error::Error for PeerManagerError {}

#[derive(Debug, Clone)]
pub struct PeerManager {
    max_peers: usize,
    sessions: BTreeMap<PeerId, PeerSession>,
    next_session_id: u64,
    events: Vec<PeerEvent>,
}

impl PeerManager {
    pub fn new(max_peers: usize) -> Self {
        Self {
            max_peers,
            sessions: BTreeMap::new(),
            next_session_id: 1,
            events: Vec::new(),
        }
    }

    pub fn connect(&mut self, peer: PeerInfo) -> Result<(), PeerManagerError> {
        if self.sessions.len() >= self.max_peers && !self.sessions.contains_key(&peer.id) {
            self.events.push(PeerEvent::RejectedMaxPeers(peer.id));
            return Err(PeerManagerError::MaxPeersReached {
                max_peers: self.max_peers,
            });
        }

        let session_id = self.allocate_session_id()?;
        let peer_id = peer.id;
        let session = PeerSession::new(session_id, peer);
        self.sessions.insert(peer_id, session);
        self.events.push(PeerEvent::Connected(peer_id));
        Ok(())
    }

    pub fn disconnect(&mut self, peer_id: &PeerId) -> bool {
        let removed = self.sessions.remove(peer_id).is_some();
        if removed {
            self.events.push(PeerEvent::Disconnected(*peer_id));
        }
        removed
    }

    pub fn peer_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn max_peers(&self) -> usize {
        self.max_peers
    }

    pub fn connected_peers(&self) -> Vec<PeerInfo> {
        self.sessions
            .values()
            .map(|session| session.peer.clone())
            .collect()
    }

    pub fn session(&self, peer_id: &PeerId) -> Option<&PeerSession> {
        self.sessions.get(peer_id)
    }

    pub fn events(&self) -> &[PeerEvent] {
        self.events.as_slice()
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    fn allocate_session_id(&mut self) -> Result<u64, PeerManagerError> {
        let session_id = self.next_session_id;
        self.next_session_id = self
            .next_session_id
            .checked_add(1)
            .ok_or(PeerManagerError::SessionIdOverflow)?;
        Ok(session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer_id(byte: u8) -> PeerId {
        [byte; 16]
    }

    #[test]
    fn connect_assigns_incrementing_session_ids_and_reconnects_in_place() {
        let mut manager = PeerManager::new(2);

        manager
            .connect(PeerInfo::new(peer_id(1), "127.0.0.1:30303"))
            .expect("first connect must succeed");
        let first_session_id = manager
            .session(&peer_id(1))
            .expect("session should exist")
            .session_id;

        manager
            .connect(PeerInfo::new(peer_id(1), "127.0.0.1:30304"))
            .expect("reconnect should replace existing session");
        let second_session_id = manager
            .session(&peer_id(1))
            .expect("updated session should exist")
            .session_id;

        assert_eq!(first_session_id, 1);
        assert_eq!(second_session_id, 2);
        assert_eq!(manager.peer_count(), 1);
        assert_eq!(
            manager.events(),
            &[
                PeerEvent::Connected(peer_id(1)),
                PeerEvent::Connected(peer_id(1))
            ]
        );
    }

    #[test]
    fn max_peers_limit_applies_only_to_new_peer_ids() {
        let mut manager = PeerManager::new(1);

        manager
            .connect(PeerInfo::new(peer_id(1), "127.0.0.1:30303"))
            .expect("first connect must succeed");

        let err = manager
            .connect(PeerInfo::new(peer_id(2), "127.0.0.1:30304"))
            .expect_err("new peer must be rejected at max peers");
        assert_eq!(err, PeerManagerError::MaxPeersReached { max_peers: 1 });

        manager
            .connect(PeerInfo::new(peer_id(1), "127.0.0.1:30305"))
            .expect("existing peer reconnect should still be allowed");

        assert_eq!(manager.peer_count(), 1);
        assert_eq!(
            manager
                .session(&peer_id(1))
                .expect("session should exist after reconnect")
                .session_id,
            2
        );
        assert_eq!(
            manager.events(),
            &[
                PeerEvent::Connected(peer_id(1)),
                PeerEvent::RejectedMaxPeers(peer_id(2)),
                PeerEvent::Connected(peer_id(1)),
            ]
        );
    }

    #[test]
    fn disconnect_is_idempotent_and_clears_session_state() {
        let mut manager = PeerManager::new(1);
        manager
            .connect(PeerInfo::new(peer_id(1), "127.0.0.1:30303"))
            .expect("connect must succeed");

        assert!(manager.disconnect(&peer_id(1)));
        assert!(!manager.disconnect(&peer_id(1)));
        assert_eq!(manager.peer_count(), 0);
        assert!(manager.session(&peer_id(1)).is_none());
        assert_eq!(
            manager.events(),
            &[
                PeerEvent::Connected(peer_id(1)),
                PeerEvent::Disconnected(peer_id(1)),
            ]
        );
    }

    #[test]
    fn connected_peers_are_returned_in_deterministic_peer_id_order() {
        let mut manager = PeerManager::new(2);
        manager
            .connect(PeerInfo::new(peer_id(2), "127.0.0.1:30304"))
            .expect("connect must succeed");
        manager
            .connect(PeerInfo::new(peer_id(1), "127.0.0.1:30303"))
            .expect("connect must succeed");

        let connected = manager.connected_peers();
        assert_eq!(connected.len(), 2);
        assert_eq!(connected[0].id, peer_id(1));
        assert_eq!(connected[1].id, peer_id(2));
    }

    #[test]
    fn connect_fails_closed_on_session_id_overflow() {
        let mut manager = PeerManager::new(1);
        manager.next_session_id = u64::MAX;

        let err = manager
            .connect(PeerInfo::new(peer_id(1), "127.0.0.1:30303"))
            .expect_err("overflow should fail before mutating state");

        assert_eq!(err, PeerManagerError::SessionIdOverflow);
        assert_eq!(manager.peer_count(), 0);
        assert!(manager.events().is_empty());
        assert!(manager.session(&peer_id(1)).is_none());
    }
}

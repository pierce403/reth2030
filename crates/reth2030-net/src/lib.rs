//! Networking and sync scaffolding for `reth2030`.
//!
//! ## Public API
//! - `PeerId`: peer identifier primitive.
//! - `PeerInfo`: peer identity and endpoint metadata.
//! - `PeerSession`: active peer session metadata.
//! - `PeerEvent`: observable peer lifecycle event stream.
//! - `PeerManager`: connect/disconnect and peer-limit manager.
//! - `PeerManagerError`: peer manager error surface.
//! - `HeaderRef`: sync header reference payload.
//! - `BlockBodyRef`: sync block-body reference payload.
//! - `SyncSource`: source trait for fetching headers and bodies.
//! - `ExecutionSink`: sink trait for executing synced blocks.
//! - `SyncStepReport`: per-block sync execution report entry.
//! - `SyncReport`: aggregate report for a sync pass.
//! - `SyncError`: sync orchestration error surface.
//! - `SyncOrchestrator`: headers -> bodies -> execution coordinator.
//! - `MockSyncSource`: deterministic in-memory sync source for tests.
//! - `RecordingExecutionSink`: deterministic in-memory execution sink.

mod peer;
mod sync;

pub use peer::{PeerEvent, PeerId, PeerInfo, PeerManager, PeerManagerError, PeerSession};
pub use sync::{
    BlockBodyRef, ExecutionSink, HeaderRef, MockSyncSource, RecordingExecutionSink, SyncError,
    SyncOrchestrator, SyncReport, SyncSource, SyncStepReport,
};

//! SPV runtime wrapper used by [`SpvBroadcaster`](crate::broadcaster::SpvBroadcaster)
//! and the platform-address sync coordinator.
//!
//! The runtime owns the connection to dashcore peers, header chain
//! state, filter sync, and transaction broadcast over P2P. It is built
//! lazily by `PlatformWalletManager::start_spv` and shared across every
//! wallet the manager owns.

mod runtime;

pub use runtime::SpvRuntime;

// Re-exports so the FFI layer can build sync configs and read progress
// without depending on `dash-spv` or `tokio-util` directly.
pub use dash_spv::sync::{
    BlockHeadersProgress, FilterHeadersProgress, FiltersProgress, MasternodesProgress,
    ProgressPercentage, SyncProgress, SyncState,
};
pub use dash_spv::ClientConfig;
pub use tokio_util::sync::CancellationToken;

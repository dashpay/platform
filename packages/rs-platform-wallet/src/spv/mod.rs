mod genesis;
mod peers;
mod runtime;

pub use genesis::{resolve_devnet_genesis_header, DevnetGenesisOverride};
pub use peers::{SpvPeerInfo, SpvPeerNodeType};
pub use runtime::SpvRuntime;

// Re-exports so the FFI layer can build sync configs and read progress
// without depending on `dash-spv` or `tokio-util` directly.
pub use dash_spv::sync::{
    BlockHeadersProgress, FilterHeadersProgress, FiltersProgress, MasternodesProgress,
    ProgressPercentage, SyncProgress, SyncState,
};
pub use dash_spv::{ClientConfig, DevnetConfig};
pub use tokio_util::sync::CancellationToken;

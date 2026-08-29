use crate::error::Error;
use crate::platform_types::platform::Platform;
use drive::grovedb::Transaction;
use std::sync::RwLock;

mod check_tx;
mod consensus;
/// Convert state transition execution result into ABCI response
pub mod execution_result;
mod full;

use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::platform_types::snapshot::{SnapshotFetchingSession, SnapshotManager};
use crate::rpc::core::DefaultCoreRPC;
#[cfg(test)]
pub(crate) use check_tx::error_into_status;
pub use check_tx::CheckTxAbciApplication;
pub use consensus::ConsensusAbciApplication;
use dpp::version::PlatformVersion;
pub use full::FullAbciApplication;

/// Platform-based ABCI application
pub trait PlatformApplication<C = DefaultCoreRPC> {
    /// Returns Platform
    fn platform(&self) -> &Platform<C>;
}

/// ABCI application that serves state sync snapshots
pub trait SnapshotManagerApplication {
    /// Returns the snapshot manager, which pins checkpoints that are actively being
    /// served so pruning cannot delete them mid-transfer
    fn snapshot_manager(&self) -> &SnapshotManager;
}

/// ABCI application that can bootstrap its state via state sync
pub trait StateSyncApplication<'p, C = DefaultCoreRPC> {
    /// Returns the state sync transfer currently in progress, if any
    fn snapshot_fetching_session(&self) -> &RwLock<Option<SnapshotFetchingSession<'p>>>;

    /// Returns Platform with the full `'p` lifetime, so a grovedb state sync session
    /// borrowing the grove can be stored in the snapshot fetching session
    fn platform(&self) -> &'p Platform<C>;
}

/// Transactional ABCI application
pub trait TransactionalApplication<'a> {
    /// Creates and keeps a new transaction
    fn start_transaction(&self);

    /// Returns the current transaction
    fn transaction(&self) -> &RwLock<Option<Transaction<'a>>>;

    /// Commits created transaction
    fn commit_transaction(&self, platform_version: &PlatformVersion) -> Result<(), Error>;
}

/// Application that executes blocks and need to keep context between handlers
pub trait BlockExecutionApplication {
    /// Returns the current block execution context
    fn block_execution_context(&self) -> &RwLock<Option<BlockExecutionContext>>;
}

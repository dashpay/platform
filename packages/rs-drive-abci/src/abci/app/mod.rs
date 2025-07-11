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
pub use check_tx::CheckTxAbciApplication;
pub use consensus::ConsensusAbciApplication;
use dpp::version::PlatformVersion;
pub use full::FullAbciApplication;

/// Platform-based ABCI application
pub trait PlatformApplication<C = DefaultCoreRPC> {
    /// Returns Platform
    fn platform(&self) -> &Platform<C>;
}

/// ABCI application supporting snapshot manager
pub trait SnapshotManagerApplication {
    /// Returns Snapshot manager
    fn snapshot_manager(&self) -> &SnapshotManager;
}

/// Transactional ABCI application
pub trait TransactionalApplication<'p> {
    /// Creates and keeps a new transaction
    fn start_transaction(&self);

    /// Returns the current transaction
    fn transaction(&self) -> &RwLock<Option<Transaction<'p>>>;

    /// Commits created transaction
    fn commit_transaction(&self, platform_version: &PlatformVersion) -> Result<(), Error>;
}

/// Application that executes blocks and need to keep context between handlers
pub trait BlockExecutionApplication {
    /// Returns the current block execution context
    fn block_execution_context(&self) -> &RwLock<Option<BlockExecutionContext>>;
}

/// Application that can maintain state sync
pub trait StateSyncApplication<'p, C = DefaultCoreRPC> {
    /// Returns the current snapshot fetching session
    fn snapshot_fetching_session(&self) -> &RwLock<Option<SnapshotFetchingSession<'p>>>;

    /// Returns platform reference
    fn platform(&self) -> &'p Platform<C>;
}

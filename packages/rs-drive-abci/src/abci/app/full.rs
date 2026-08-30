use crate::abci::app::{
    BlockExecutionApplication, PlatformApplication, SnapshotManagerApplication,
    StateSyncApplication, TransactionalApplication,
};
use crate::abci::handler;
use crate::abci::handler::error::error_into_exception;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::platform_types::platform::Platform;
use crate::platform_types::snapshot::{SnapshotFetchingSession, SnapshotManager};
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use std::fmt::Debug;
use std::sync::RwLock;
use tenderdash_abci::proto::abci as proto;

/// AbciApp is an implementation of ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
pub struct FullAbciApplication<'a, C> {
    /// Platform
    pub platform: &'a Platform<C>,
    /// The current GroveDB transaction
    pub transaction: RwLock<Option<Transaction<'a>>>,
    /// The current block execution context
    pub block_execution_context: RwLock<Option<BlockExecutionContext>>,
    /// The snapshot manager, pinning checkpoints that are being served to peers
    pub snapshot_manager: SnapshotManager,
    /// The state sync transfer currently in progress, if any
    pub snapshot_fetching_session: RwLock<Option<SnapshotFetchingSession<'a>>>,
}

impl<'a, C> FullAbciApplication<'a, C> {
    /// Create new ABCI app
    pub fn new(platform: &'a Platform<C>) -> Self {
        Self {
            platform,
            transaction: Default::default(),
            block_execution_context: Default::default(),
            snapshot_manager: SnapshotManager::new(),
            snapshot_fetching_session: Default::default(),
        }
    }
}

impl<C> PlatformApplication<C> for FullAbciApplication<'_, C> {
    fn platform(&self) -> &Platform<C> {
        self.platform
    }
}

impl<C> SnapshotManagerApplication for FullAbciApplication<'_, C> {
    fn snapshot_manager(&self) -> &SnapshotManager {
        &self.snapshot_manager
    }
}

impl<'a, C> StateSyncApplication<'a, C> for FullAbciApplication<'a, C> {
    fn snapshot_fetching_session(&self) -> &RwLock<Option<SnapshotFetchingSession<'a>>> {
        &self.snapshot_fetching_session
    }

    fn platform(&self) -> &'a Platform<C> {
        self.platform
    }
}

impl<C> BlockExecutionApplication for FullAbciApplication<'_, C> {
    fn block_execution_context(&self) -> &RwLock<Option<BlockExecutionContext>> {
        &self.block_execution_context
    }
}

impl<'a, C> TransactionalApplication<'a> for FullAbciApplication<'a, C> {
    /// create and store a new transaction
    fn start_transaction(&self) {
        let transaction = self.platform.drive.grove.start_transaction();
        self.transaction.write().unwrap().replace(transaction);
    }

    fn transaction(&self) -> &RwLock<Option<Transaction<'a>>> {
        &self.transaction
    }

    /// Commit a transaction
    fn commit_transaction(&self, platform_version: &PlatformVersion) -> Result<(), Error> {
        let transaction = self
            .transaction
            .write()
            .unwrap()
            .take()
            .ok_or(Error::Execution(ExecutionError::NotInTransaction(
                "trying to commit a transaction, but we are not in one",
            )))?;

        self.platform
            .drive
            .commit_transaction(transaction, &platform_version.drive)
            .map_err(Error::Drive)
    }
}

impl<C> Debug for FullAbciApplication<'_, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<FullAbciApplication>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;

    #[test]
    fn full_abci_application_debug_format() {
        let platform =
            crate::test::helpers::setup::TestPlatformBuilder::new().build_with_mock_rpc();

        let app = FullAbciApplication::new(&platform.platform);

        let debug_str = format!("{:?}", app);
        assert_eq!(debug_str, "<FullAbciApplication>");
    }

    #[test]
    fn full_abci_application_platform_returns_reference() {
        let platform =
            crate::test::helpers::setup::TestPlatformBuilder::new().build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        let _platform_ref = PlatformApplication::platform(&app);
    }

    #[test]
    fn full_abci_application_block_execution_context_is_initially_none() {
        let platform =
            crate::test::helpers::setup::TestPlatformBuilder::new().build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        let ctx = app.block_execution_context().read().unwrap();
        assert!(ctx.is_none());
    }

    #[test]
    fn full_abci_application_transaction_is_initially_none() {
        let platform =
            crate::test::helpers::setup::TestPlatformBuilder::new().build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        let tx = app.transaction().read().unwrap();
        assert!(tx.is_none());
    }

    #[test]
    fn full_abci_application_start_transaction_creates_transaction() {
        let platform =
            crate::test::helpers::setup::TestPlatformBuilder::new().build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        app.start_transaction();

        let tx = app.transaction().read().unwrap();
        assert!(tx.is_some());
    }

    #[test]
    fn full_abci_application_commit_without_transaction_fails() {
        let platform =
            crate::test::helpers::setup::TestPlatformBuilder::new().build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        let platform_version = PlatformVersion::latest();

        let result = app.commit_transaction(platform_version);
        assert!(result.is_err());
    }

    #[test]
    fn full_abci_application_start_and_commit_transaction_succeeds() {
        let platform =
            crate::test::helpers::setup::TestPlatformBuilder::new().build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        let platform_version = PlatformVersion::latest();

        app.start_transaction();

        let result = app.commit_transaction(platform_version);
        assert!(result.is_ok());

        // After commit, transaction should be consumed
        let tx = app.transaction().read().unwrap();
        assert!(tx.is_none());
    }
}

impl<C> tenderdash_abci::Application for FullAbciApplication<'_, C>
where
    C: CoreRPCLike,
{
    fn info(
        &self,
        request: proto::RequestInfo,
    ) -> Result<proto::ResponseInfo, proto::ResponseException> {
        handler::info(self, request).map_err(error_into_exception)
    }

    fn init_chain(
        &self,
        request: proto::RequestInitChain,
    ) -> Result<proto::ResponseInitChain, proto::ResponseException> {
        handler::init_chain(self, request).map_err(error_into_exception)
    }

    fn query(
        &self,
        _request: proto::RequestQuery,
    ) -> Result<proto::ResponseQuery, proto::ResponseException> {
        unreachable!("query is not supported in full ABCI application")
    }

    fn check_tx(
        &self,
        request: proto::RequestCheckTx,
    ) -> Result<proto::ResponseCheckTx, proto::ResponseException> {
        handler::check_tx(self.platform, &self.platform.core_rpc, request)
            .map_err(error_into_exception)
    }

    fn extend_vote(
        &self,
        request: proto::RequestExtendVote,
    ) -> Result<proto::ResponseExtendVote, proto::ResponseException> {
        handler::extend_vote(self, request).map_err(error_into_exception)
    }

    fn finalize_block(
        &self,
        request: proto::RequestFinalizeBlock,
    ) -> Result<proto::ResponseFinalizeBlock, proto::ResponseException> {
        // Autonomous expiry of serving pins: an abandoned state sync transfer stops
        // making chunk requests, so nothing on the serving path would ever release its
        // pin and the pruned checkpoint directory would stay on disk forever. A block is
        // the one thing that reliably keeps happening. This is node-local bookkeeping and
        // touches no consensus state.
        self.snapshot_manager.release_expired_pins();

        handler::finalize_block(self, request).map_err(error_into_exception)
    }

    fn prepare_proposal(
        &self,
        request: proto::RequestPrepareProposal,
    ) -> Result<proto::ResponsePrepareProposal, proto::ResponseException> {
        handler::prepare_proposal(self, request).map_err(error_into_exception)
    }

    fn process_proposal(
        &self,
        request: proto::RequestProcessProposal,
    ) -> Result<proto::ResponseProcessProposal, proto::ResponseException> {
        handler::process_proposal(self, request).map_err(error_into_exception)
    }

    fn verify_vote_extension(
        &self,
        request: proto::RequestVerifyVoteExtension,
    ) -> Result<proto::ResponseVerifyVoteExtension, proto::ResponseException> {
        handler::verify_vote_extension(self, request).map_err(error_into_exception)
    }

    fn list_snapshots(
        &self,
        request: proto::RequestListSnapshots,
    ) -> Result<proto::ResponseListSnapshots, proto::ResponseException> {
        handler::list_snapshots(self, request).map_err(error_into_exception)
    }

    fn load_snapshot_chunk(
        &self,
        request: proto::RequestLoadSnapshotChunk,
    ) -> Result<proto::ResponseLoadSnapshotChunk, proto::ResponseException> {
        handler::load_snapshot_chunk(self, request).map_err(error_into_exception)
    }

    fn offer_snapshot(
        &self,
        request: proto::RequestOfferSnapshot,
    ) -> Result<proto::ResponseOfferSnapshot, proto::ResponseException> {
        handler::offer_snapshot(self, request).map_err(error_into_exception)
    }

    fn apply_snapshot_chunk(
        &self,
        request: proto::RequestApplySnapshotChunk,
    ) -> Result<proto::ResponseApplySnapshotChunk, proto::ResponseException> {
        handler::apply_snapshot_chunk(self, request).map_err(error_into_exception)
    }
}

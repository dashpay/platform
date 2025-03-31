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
    /// The State sync session
    pub snapshot_fetching_session: RwLock<Option<SnapshotFetchingSession<'a>>>,
    /// The snapshot manager
    pub snapshot_manager: SnapshotManager,
}

impl<'a, C> FullAbciApplication<'a, C> {
    /// Create new ABCI app
    pub fn new(platform: &'a Platform<C>) -> Self {
        let snapshot_manager = SnapshotManager::new(
            platform.config.state_sync_config.checkpoints_path.clone(),
            platform.config.state_sync_config.max_num_snapshots,
            platform.config.state_sync_config.snapshots_frequency,
        );
        Self {
            platform,
            transaction: Default::default(),
            block_execution_context: Default::default(),
            snapshot_fetching_session: Default::default(),
            snapshot_manager,
        }
    }
}

impl<C> PlatformApplication<C> for FullAbciApplication<'_, C> {
    fn platform(&self) -> &Platform<C> {
        self.platform
    }
}

impl<'a, C> SnapshotManagerApplication for FullAbciApplication<'a, C> {
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

impl<'a, C> BlockExecutionApplication for FullAbciApplication<'a, C> {
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
}

use crate::abci::app::{
    BlockExecutionApplication, PlatformApplication, SnapshotManagerApplication,
    StateSyncApplication, TransactionalApplication,
};
use crate::abci::handler::error::error_into_exception;
use crate::abci::{handler, AbciError};
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::platform_types::platform::Platform;
use crate::platform_types::snapshot::{SnapshotFetchingSession, SnapshotManager};
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::grovedb::replication::{MultiStateSyncSession, CURRENT_STATE_SYNC_VERSION};
use drive::grovedb::Transaction;
use std::fmt::Debug;
use std::pin::Pin;
use std::sync::{LockResult, RwLock};
use tenderdash_abci::proto::abci as proto;

/// AbciApp is an implementation of ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
/// 'p: 'tx, means that Platform must outlive the transaction
pub struct ConsensusAbciApplication<'p, C> {
    /// Platform
    platform: &'p Platform<C>,
    /// The current GroveDb transaction
    transaction: RwLock<Option<Transaction<'p>>>,
    /// The current block execution context
    block_execution_context: RwLock<Option<BlockExecutionContext>>,
    /// The State sync session
    snapshot_fetching_session: RwLock<Option<SnapshotFetchingSession<'p>>>,
    /// The snapshot manager
    snapshot_manager: SnapshotManager,
}

impl<'p, C> ConsensusAbciApplication<'p, C> {
    /// Create new ABCI app
    pub fn new(platform: &'p Platform<C>) -> Self {
        Self {
            platform,
            transaction: Default::default(),
            block_execution_context: Default::default(),
            snapshot_fetching_session: Default::default(),
            snapshot_manager: Default::default(),
        }
    }
}

impl<'p, C> PlatformApplication<C> for ConsensusAbciApplication<'p, C> {
    fn platform(&self) -> &Platform<C> {
        self.platform
    }
}

impl<'p, C> SnapshotManagerApplication for ConsensusAbciApplication<'p, C> {
    fn snapshot_manager(&self) -> &SnapshotManager {
        &self.snapshot_manager
    }
}

impl<'p, C> StateSyncApplication<'p, C> for ConsensusAbciApplication<'p, C> {
    fn snapshot_fetching_session(&self) -> &RwLock<Option<SnapshotFetchingSession<'p>>> {
        &self.snapshot_fetching_session
    }

    fn platform(&self) -> &'p Platform<C> {
        self.platform
    }
}

impl<'p, C> BlockExecutionApplication for ConsensusAbciApplication<'p, C> {
    fn block_execution_context(&self) -> &RwLock<Option<BlockExecutionContext>> {
        &self.block_execution_context
    }
}

impl<'p, C> TransactionalApplication<'p> for ConsensusAbciApplication<'p, C> {
    /// create and store a new transaction
    fn start_transaction(&self) {
        let transaction = self.platform.drive.grove.start_transaction();
        self.transaction.write().unwrap().replace(transaction);
    }

    fn transaction(&self) -> &RwLock<Option<Transaction<'p>>> {
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

impl<'p, C> Debug for ConsensusAbciApplication<'p, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<ConsensusAbciApplication>")
    }
}

impl<'p, C> tenderdash_abci::Application for ConsensusAbciApplication<'p, C>
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
        unreachable!("query is not implemented for consensus ABCI application")
    }

    fn check_tx(
        &self,
        _request: proto::RequestCheckTx,
    ) -> Result<proto::ResponseCheckTx, proto::ResponseException> {
        unreachable!("check_tx is not implemented for consensus ABCI application")
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
        if request.app_hash.len() != 32 {
            return Err(error_into_exception(Error::Abci(AbciError::BadRequest(
                "offer_snapshot invalid app_hash in request".to_string(),
            ))));
        }

        let mut request_app_hash = [0u8; 32];
        request_app_hash.copy_from_slice(&request.app_hash);

        match request.snapshot {
            None => Err(error_into_exception(Error::Abci(AbciError::BadRequest(
                "offer_snapshot missing snapshot in request".to_string(),
            )))),
            Some(offered_snapshot) => {
                match self.snapshot_fetching_session.write() {
                    Ok(mut session_write) => {
                        // Now `session_write` is a mutable reference to the inner data
                        match *session_write {
                            Some(ref mut session) => {
                                // Access and modify `session` here
                                if offered_snapshot.height <= session.snapshot.height {
                                    return Err(error_into_exception(Error::Abci(
                                        AbciError::BadRequest(
                                            "offer_snapshot already syncing newest height"
                                                .to_string(),
                                        ),
                                    )));
                                }

                                match self.platform.drive.grove.wipe() {
                                    Ok(_) => {
                                        let response = proto::ResponseOfferSnapshot::default();

                                        match self.platform.drive.grove.start_snapshot_syncing(request_app_hash, CURRENT_STATE_SYNC_VERSION) {
                                            Ok((_, state_sync_info)) => {
                                                session.snapshot = offered_snapshot;
                                                session.app_hash = request.app_hash;
                                                session.state_sync_info = state_sync_info;

                                                Ok(response)
                                            }
                                            Err(e) => Err(error_into_exception(Error::Abci(
                                                AbciError::BadRequest(format!(
                                                    "offer_snapshot unable start_snapshot_syncing:{}",
                                                    e
                                                )),
                                            ))),
                                        }
                                    }
                                    Err(e) => Err(error_into_exception(Error::Abci(
                                        AbciError::BadRequest(format!(
                                            "offer_snapshot unable to wipe grovedb:{}",
                                            e
                                        )),
                                    ))),
                                }
                            }
                            None => Err(error_into_exception(Error::Abci(AbciError::BadRequest(
                                "offer_snapshot unable to lock session".to_string(),
                            )))),
                        }
                    }
                    Err(_poisoned) => {
                        Err(error_into_exception(Error::Abci(AbciError::BadRequest(
                            "offer_snapshot unable to lock session (poisoned)".to_string(),
                        ))))
                    }
                }
            }
        }
    }

    fn apply_snapshot_chunk(
        &self,
        request: proto::RequestApplySnapshotChunk,
    ) -> Result<proto::ResponseApplySnapshotChunk, proto::ResponseException> {
        handler::apply_snapshot_chunk(self, request).map_err(error_into_exception)
    }
}

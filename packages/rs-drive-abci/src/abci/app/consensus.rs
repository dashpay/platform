use crate::abci::app::block_update::BlockUpdateChannel;
use crate::abci::app::{NamedApplication, PlatformApplication, TransactionalApplication};
use crate::abci::handler;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use drive::grovedb::Transaction;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};
use tenderdash_abci::proto::abci as proto;

/// AbciApp is an implementation of ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
pub struct ConsensusAbciApplication<'a, C> {
    // TODO: Why we do not own platform?
    /// Platform
    platform: &'a Platform<C>,
    /// The current transaction
    transaction: RwLock<Option<Transaction<'a>>>,
    block_update_channel: Arc<BlockUpdateChannel>,
}

impl<'a, C> NamedApplication for ConsensusAbciApplication<'a, C> {
    fn name(&self) -> String {
        String::from("Consensus")
    }
}

impl<'a, C> ConsensusAbciApplication<'a, C> {
    /// Create new ABCI app
    pub fn new(
        platform: &'a Platform<C>,
        block_update_channel: Arc<BlockUpdateChannel>,
    ) -> Result<Self, Error> {
        let app = Self {
            platform,
            transaction: RwLock::new(None),
            block_update_channel,
        };

        Ok(app)
    }
}

impl<'a, C> PlatformApplication<C> for ConsensusAbciApplication<'a, C> {
    fn platform(&self) -> &Platform<C> {
        self.platform
    }
}

impl<'a, C> TransactionalApplication<'a> for ConsensusAbciApplication<'a, C> {
    /// create and store a new transaction
    fn start_transaction(&self) {
        let transaction = self.platform.drive.grove.start_transaction();
        self.transaction.write().unwrap().replace(transaction);
    }

    fn transaction(&self) -> &RwLock<Option<Transaction<'a>>> {
        &self.transaction
    }

    /// Commit a transaction
    fn commit_transaction(&self) -> Result<(), Error> {
        let transaction = self
            .transaction
            .write()
            .unwrap()
            .take()
            .ok_or(Error::Execution(ExecutionError::NotInTransaction(
                "trying to commit a transaction, but we are not in one",
            )))?;
        let platform_state = self.platform.state.read().unwrap();
        let platform_version = platform_state.current_platform_version()?;
        self.platform
            .drive
            .commit_transaction(transaction, &platform_version.drive)
            .map_err(Error::Drive)
    }
}

impl<'a, C> Debug for ConsensusAbciApplication<'a, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<ConsensusAbciApplication>")
    }
}

impl<'a, C> tenderdash_abci::Application for ConsensusAbciApplication<'a, C>
where
    C: CoreRPCLike,
{
    fn info(
        &self,
        request: proto::RequestInfo,
    ) -> Result<proto::ResponseInfo, proto::ResponseException> {
        handler::info(self, request)
    }

    fn init_chain(
        &self,
        request: proto::RequestInitChain,
    ) -> Result<proto::ResponseInitChain, proto::ResponseException> {
        handler::init_chain(self, request)
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
        handler::extend_vote(self, request)
    }

    fn finalize_block(
        &self,
        request: proto::RequestFinalizeBlock,
    ) -> Result<proto::ResponseFinalizeBlock, proto::ResponseException> {
        // Collect Data Contract block cache
        let drive_cache = self.platform.drive.cache.read().unwrap();
        let data_contracts_block_cache = drive_cache.cached_contracts.block_cache().clone();
        drop(drive_cache);

        let response = handler::finalize_block(self, request);

        if response.is_ok() {
            // Send state cache and data contract block cache to Query App thread
            let state_cache = self.platform.state.read().unwrap();

            self.block_update_channel
                .update(data_contracts_block_cache, state_cache.clone());
        }

        response
    }

    fn prepare_proposal(
        &self,
        request: proto::RequestPrepareProposal,
    ) -> Result<proto::ResponsePrepareProposal, proto::ResponseException> {
        handler::prepare_proposal(self, request)
    }

    fn process_proposal(
        &self,
        request: proto::RequestProcessProposal,
    ) -> Result<proto::ResponseProcessProposal, proto::ResponseException> {
        handler::process_proposal(self, request)
    }

    fn verify_vote_extension(
        &self,
        request: proto::RequestVerifyVoteExtension,
    ) -> Result<proto::ResponseVerifyVoteExtension, proto::ResponseException> {
        handler::verify_vote_extension(self, request)
    }
}

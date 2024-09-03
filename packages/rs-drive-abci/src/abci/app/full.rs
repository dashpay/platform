use crate::abci::app::{BlockExecutionApplication, PlatformApplication, TransactionalApplication};
use crate::abci::handler;
use crate::abci::handler::error::error_into_exception;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use std::fmt::Debug;
use std::sync::RwLock;
use tenderdash_abci::proto::abci::{self as proto};

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
}

impl<'a, C> FullAbciApplication<'a, C> {
    /// Create new ABCI app
    pub fn new(platform: &'a Platform<C>) -> Self {
        Self {
            platform,
            transaction: Default::default(),
            block_execution_context: Default::default(),
        }
    }

    /// Dump request and response to the dump directory
    ///
    /// This function is used for debugging purposes. It dumps the request and response to the dump directory if it's set.
    /// File name format is: <height>_<round>_<request_type>_<timestamp>.json
    /// If height or round is not available, both will be skipped and the file name will be <request_type>_<timestamp>.json
    ///
    /// ## Parameters
    ///
    /// - `req`: The request to dump
    /// - `resp`: The response to dump
    /// - `height`: The height of the block
    /// - `round`: The round of the block
    ///
    /// ## Returns
    ///
    /// The response, for easier chaining
    ///
    /// ## Errors
    ///
    /// Errors are logged at `debug` level and ignored
    fn dump_req_resp<T: serde::Serialize, U: serde::Serialize>(
        &self,
        req: T,
        resp: Result<U, proto::ResponseException>,
        height: Option<i64>,
        round: Option<i32>,
    ) -> Result<U, proto::ResponseException> {
        if let Some(dump_dir) = self.platform.config.abci.dump_dir.as_ref() {
            let now = chrono::Utc::now().timestamp();

            dump(&req, dump_dir, height, round, now).unwrap_or_else(|e| {
                tracing::error!("failed to dump request: {:?}", e);
            });

            match resp {
                Ok(ref r) => dump(&r, dump_dir, height, round, now).unwrap_or_else(|e| {
                    tracing::error!("failed to dump response: {:?}", e);
                }),
                // TODO: implement serde for ResponseException
                Err(ref e) => dump(&e.error, dump_dir, height, round, now).unwrap_or_else(|e| {
                    tracing::error!("failed to dump response: {:?}", e);
                }),
            }
        };

        resp
    }
}

/// Dump serializable data to the dump directory
///
/// ## Parameters
///
/// - `req`: The serializable data to dump
/// - `dump_dir`: The directory to dump the data to
/// - `height`: The height of the block
/// - `round`: The round of the block
/// - `now`: The current timestamp
///
/// ## Returns
///
/// Result<(), std::io::Error>
fn dump<T: serde::Serialize>(
    msg: &T,
    dump_dir: &std::path::Path,
    height: Option<i64>,
    round: Option<i32>,
    now: i64,
) -> Result<(), std::io::Error> {
    let file = if let (Some(h), Some(r)) = (height, round) {
        dump_dir.join(format!(
            "{}_{}_{}_{}.json",
            h,
            r,
            std::any::type_name::<T>(),
            now,
        ))
    } else {
        dump_dir.join(format!("{}_{}.json", std::any::type_name::<T>(), now,))
    };

    let writer = std::fs::File::create(&file)?;
    // Dump request
    serde_json::to_writer(writer, msg)?;
    Ok(())
}

/// Macro that dumps request and response to the dump directory

impl<'a, C> PlatformApplication<C> for FullAbciApplication<'a, C> {
    fn platform(&self) -> &Platform<C> {
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

impl<'a, C> Debug for FullAbciApplication<'a, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<FullAbciApplication>")
    }
}

impl<'a, C> tenderdash_abci::Application for FullAbciApplication<'a, C>
where
    C: CoreRPCLike,
{
    fn info(
        &self,
        request: proto::RequestInfo,
    ) -> Result<proto::ResponseInfo, proto::ResponseException> {
        self.dump_req_resp(
            &request,
            handler::info(self, request).map_err(error_into_exception),
            None,
            None,
        )
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
        self.dump_req_resp(
            request,
            handler::finalize_block(self, request).map_err(error_into_exception),
            Some(request.height),
            Some(request.round),
        )
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
}

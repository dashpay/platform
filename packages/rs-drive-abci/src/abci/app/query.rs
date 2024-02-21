use crate::abci::app::{NamedApplication, PlatformApplication};
use crate::abci::handler;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use std::fmt::Debug;
use tenderdash_abci::proto::abci as proto;

/// AbciApp is an implementation of ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
pub struct QueryAbciApplication<'a, C> {
    /// Platform
    platform: &'a Platform<C>,
}

impl<'a, C> NamedApplication for QueryAbciApplication<'a, C> {
    fn name(&self) -> String {
        String::from("Query")
    }
}

impl<'a, C> PlatformApplication<C> for QueryAbciApplication<'a, C> {
    fn platform(&self) -> &Platform<C> {
        self.platform
    }
}

impl<'a, C> QueryAbciApplication<'a, C> {
    /// Create new ABCI app
    pub fn new(platform: &'a Platform<C>) -> Result<QueryAbciApplication<'a, C>, Error> {
        let app = Self { platform };

        Ok(app)
    }
}

impl<'a, C> Debug for QueryAbciApplication<'a, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<QueryAbciApplication>")
    }
}

impl<'a, C> tenderdash_abci::Application for QueryAbciApplication<'a, C>
where
    C: CoreRPCLike,
{
    fn info(
        &self,
        _request: proto::RequestInfo,
    ) -> Result<proto::ResponseInfo, proto::ResponseException> {
        unreachable!("info is not implemented for read-only ABCI application")
    }

    fn init_chain(
        &self,
        _request: proto::RequestInitChain,
    ) -> Result<proto::ResponseInitChain, proto::ResponseException> {
        unreachable!("init_chain is not implemented for read-only ABCI application")
    }

    fn query(
        &self,
        request: proto::RequestQuery,
    ) -> Result<proto::ResponseQuery, proto::ResponseException> {
        unreachable!("query is not implemented for Query ABCI application")
    }

    fn check_tx(
        &self,
        request: proto::RequestCheckTx,
    ) -> Result<proto::ResponseCheckTx, proto::ResponseException> {
        handler::check_tx(self, request)
    }

    fn extend_vote(
        &self,
        _request: proto::RequestExtendVote,
    ) -> Result<proto::ResponseExtendVote, proto::ResponseException> {
        unreachable!("extend_vote is not implemented for read-only ABCI application")
    }

    fn finalize_block(
        &self,
        _request: proto::RequestFinalizeBlock,
    ) -> Result<proto::ResponseFinalizeBlock, proto::ResponseException> {
        unreachable!("extend_vote is unreachable")
    }

    fn prepare_proposal(
        &self,
        _request: proto::RequestPrepareProposal,
    ) -> Result<proto::ResponsePrepareProposal, proto::ResponseException> {
        unreachable!("prepare_proposal is not implemented for read-only ABCI application")
    }

    fn process_proposal(
        &self,
        _request: proto::RequestProcessProposal,
    ) -> Result<proto::ResponseProcessProposal, proto::ResponseException> {
        unreachable!("process_proposal is not implemented for read-only ABCI application")
    }

    fn verify_vote_extension(
        &self,
        _request: proto::RequestVerifyVoteExtension,
    ) -> Result<proto::ResponseVerifyVoteExtension, proto::ResponseException> {
        unreachable!("verify_vote_extension is not implemented for read-only ABCI application")
    }
}

use crate::abci::app::block_update::BlockUpdateChannel;
use crate::abci::app::{NamedApplication, PlatformApplication};
use crate::abci::handler;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use dpp::version::PlatformVersionCurrentVersion;
use std::fmt::Debug;
use std::sync::Arc;
use tenderdash_abci::proto::abci as proto;

/// AbciApp is an implementation of ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
pub struct QueryAbciApplication<'a, C> {
    /// Platform
    platform: &'a Platform<C>,
    block_update_channel: Arc<BlockUpdateChannel>,
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
    pub fn new(
        platform: &'a Platform<C>,
        block_update_channel: Arc<BlockUpdateChannel>,
    ) -> Result<QueryAbciApplication<'a, C>, Error> {
        let app = Self {
            platform,
            block_update_channel,
        };

        Ok(app)
    }

    fn receive_and_apply_block_update(&self) {
        let Some(block_update) = self.block_update_channel.receive() else {
            return;
        };

        let time = std::time::Instant::now();

        // Update platform state cache
        let mut state_cache = self.platform.state.write().unwrap();
        *state_cache = block_update.platform_state;
        let current_protocol_version = state_cache.current_protocol_version_in_consensus();
        let height = state_cache.last_committed_block_height();
        drop(state_cache);

        // Update data contract cache
        let mut drive_cache = self.platform.drive.cache.write().unwrap();
        drive_cache
            .cached_contracts
            .consume(block_update.data_contracts_cache);
        drop(drive_cache);

        // Catchup to new updated state
        self.platform
            .drive
            .grove
            .try_to_catch_up_from_primary()
            .expect("failed to catch up");

        // TODO: Do we need this or it will be already updated from consensus thread?
        // Update version
        let platform_version = PlatformVersion::get(current_protocol_version)
            .expect("must be present since used by consensus thread");

        PlatformVersion::set_current(platform_version);

        tracing::debug!(
            "Received and applied block update from consensus app for height {}, took {} ms",
            height,
            time.elapsed().as_millis()
        );
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
        handler::query(self, request)
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
        self.receive_and_apply_block_update();

        Ok(proto::ResponseFinalizeBlock {
            events: vec![],
            retain_height: 0,
        })
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

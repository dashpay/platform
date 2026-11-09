//! Blocks of the Platform chain and the state transitions in them (`getBlocks`,
//! `getOwnerTransactionsInBlock`). Nodes serve both from protocol version 15, and the SDK
//! sends no such request before the network has proven 15.
//!
//! [`Sdk::fetch_blocks`] reads up to ten consecutive blocks, each with every state
//! transition it holds and the result of its execution. [`Sdk::fetch_owner_transactions_in_block`]
//! reads the state transitions one identity owns in one block.
//!
//! These answers are about blocks, which Platform's state tree does not hold, so what
//! proves them is not a GroveDB proof: it is the block itself, as Tenderdash's validator
//! quorum signed it. With proofs on (the SDK's default) every block must carry the quorum's
//! signature over its header, checked with the key the context provider supplies, and the
//! transactions and results must hash to the two roots in that header. See [`Blocks`] and
//! [`OwnerTransactionsInBlock`] for what that leaves open: a proved block says nothing
//! about how recent it is, and the list of an owner's transactions is not proved complete.
//!
//! With proofs off ([`SdkBuilder::with_proofs`](crate::SdkBuilder::with_proofs)) the answer
//! is the word of the node that gave it.

use crate::platform::{Identifier, Query, QuerySettings};
use crate::sdk::quorum_key::{verify_fetching_quorum_keys, KeyFetchFailure};
use crate::sync::retry;
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::get_blocks_request::GetBlocksRequestV0;
use dapi_grpc::platform::v0::get_owner_transactions_in_block_request::GetOwnerTransactionsInBlockRequestV0;
use dapi_grpc::platform::v0::{
    get_blocks_request, get_owner_transactions_in_block_request, GetBlocksRequest,
    GetBlocksResponse, GetOwnerTransactionsInBlockRequest, GetOwnerTransactionsInBlockResponse,
};
use dash_context_provider::ContextProvider;
use dpp::version::PlatformVersion;
pub use drive_proof_verifier::types::blocks::{
    Block, BlockTransaction, Blocks, OwnerTransactionsInBlock,
};
use rs_dapi_client::transport::TransportRequest;
use rs_dapi_client::RequestSettings;
use rs_dapi_client::{DapiRequest, ExecutionError, ExecutionResponse, InnerInto, IntoInner};

/// Query for the state transitions one identity owns in one block.
///
/// The owner of a state transition is the identity it reports as its own. A recipient is
/// not an owner, and transitions funded from addresses or the shielded pool have none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerTransactionsInBlockQuery {
    /// The height of the block, from 1.
    pub height: u64,
    /// The identity whose state transitions to return.
    pub owner_id: Identifier,
}

impl Query<GetOwnerTransactionsInBlockRequest> for OwnerTransactionsInBlockQuery {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetOwnerTransactionsInBlockRequest, Error> {
        Ok(GetOwnerTransactionsInBlockRequest {
            version: Some(get_owner_transactions_in_block_request::Version::V0(
                GetOwnerTransactionsInBlockRequestV0 {
                    height: self.height,
                    owner_id: self.owner_id.to_vec(),
                    prove: settings.prove,
                },
            )),
        })
    }
}

/// Query for consecutive blocks from a start height.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlocksQuery {
    /// The height of the first block, from 1.
    pub start_height: u64,
    /// How many blocks to return, from 1 to 10.
    pub count: u32,
}

impl Query<GetBlocksRequest> for BlocksQuery {
    fn query(&self, settings: &QuerySettings<'_>) -> Result<GetBlocksRequest, Error> {
        Ok(GetBlocksRequest {
            version: Some(get_blocks_request::Version::V0(GetBlocksRequestV0 {
                start_height: self.start_height,
                count: self.count,
                prove: settings.prove,
            })),
        })
    }
}

/// A request answered with Tenderdash block proofs in place of a GroveDB proof and
/// response metadata, which is why it does not go through [`Fetch`](crate::platform::Fetch).
trait BlockRequest: TransportRequest + Clone {
    /// What the request's answer is read into.
    type Answer: Send;

    /// How many blocks an answer can hold. Another quorum may have signed each.
    fn max_blocks(&self) -> usize;

    /// Verify the answer to this request, which asked to prove.
    fn proved_answer(
        &self,
        response: Self::Response,
        provider: &dyn ContextProvider,
    ) -> Result<Self::Answer, drive_proof_verifier::Error>;

    /// Read the answer to this request, which did not ask to prove.
    fn unproved_answer(
        &self,
        response: Self::Response,
        platform_version: &PlatformVersion,
    ) -> Result<Self::Answer, drive_proof_verifier::Error>;
}

impl BlockRequest for GetOwnerTransactionsInBlockRequest {
    type Answer = OwnerTransactionsInBlock;

    fn max_blocks(&self) -> usize {
        1
    }

    fn proved_answer(
        &self,
        response: GetOwnerTransactionsInBlockResponse,
        provider: &dyn ContextProvider,
    ) -> Result<Self::Answer, drive_proof_verifier::Error> {
        OwnerTransactionsInBlock::from_proved_response(self, response, provider)
    }

    fn unproved_answer(
        &self,
        response: GetOwnerTransactionsInBlockResponse,
        platform_version: &PlatformVersion,
    ) -> Result<Self::Answer, drive_proof_verifier::Error> {
        OwnerTransactionsInBlock::from_unproved_response(self, response, platform_version)
    }
}

impl BlockRequest for GetBlocksRequest {
    type Answer = Blocks;

    fn max_blocks(&self) -> usize {
        match &self.version {
            Some(get_blocks_request::Version::V0(v0)) => {
                usize::try_from(v0.count).unwrap_or(usize::MAX)
            }
            None => 0,
        }
    }

    fn proved_answer(
        &self,
        response: GetBlocksResponse,
        provider: &dyn ContextProvider,
    ) -> Result<Self::Answer, drive_proof_verifier::Error> {
        Blocks::from_proved_response(self, response, provider)
    }

    fn unproved_answer(
        &self,
        response: GetBlocksResponse,
        platform_version: &PlatformVersion,
    ) -> Result<Self::Answer, drive_proof_verifier::Error> {
        Blocks::from_unproved_response(self, response, platform_version)
    }
}

impl Sdk {
    /// Fetch the state transitions `query.owner_id` owns in the block at `query.height`,
    /// each with the result of its execution.
    ///
    /// With proofs on, every returned transition is proved to be in that block, with that
    /// result, and to be the owner's. The list is not proved complete; see
    /// [`OwnerTransactionsInBlock`]. A block that holds so many of the owner's transitions
    /// that the proved answer would pass about 4 MB is refused by the node: read it with
    /// [`Sdk::fetch_blocks`] instead.
    pub async fn fetch_owner_transactions_in_block(
        &self,
        query: OwnerTransactionsInBlockQuery,
    ) -> Result<OwnerTransactionsInBlock, Error> {
        self.fetch_owner_transactions_in_block_with_settings(query, RequestSettings::default())
            .await
    }

    /// [`Sdk::fetch_owner_transactions_in_block`] with custom request settings.
    pub async fn fetch_owner_transactions_in_block_with_settings(
        &self,
        query: OwnerTransactionsInBlockQuery,
        settings: RequestSettings,
    ) -> Result<OwnerTransactionsInBlock, Error> {
        self.fetch_block_answer::<GetOwnerTransactionsInBlockRequest, _>(query, settings)
            .await
    }

    /// Fetch up to `query.count` consecutive blocks from `query.start_height`, each with
    /// every state transition it holds and the result of its execution.
    ///
    /// With proofs on, each returned block is proved to be the block a validator quorum
    /// committed at its height, with exactly those transitions and results. There can be
    /// fewer blocks than asked for; see [`Blocks`]. Ask from the height after the last one
    /// to continue.
    pub async fn fetch_blocks(&self, query: BlocksQuery) -> Result<Blocks, Error> {
        self.fetch_blocks_with_settings(query, RequestSettings::default())
            .await
    }

    /// [`Sdk::fetch_blocks`] with custom request settings.
    pub async fn fetch_blocks_with_settings(
        &self,
        query: BlocksQuery,
        settings: RequestSettings,
    ) -> Result<Blocks, Error> {
        self.fetch_block_answer::<GetBlocksRequest, _>(query, settings)
            .await
    }

    /// Send the request `query` encodes and read its answer: verified against the block
    /// proofs it carries when the SDK uses proofs, as the node gave it otherwise.
    async fn fetch_block_answer<R: BlockRequest, Q: Query<R>>(
        &self,
        query: Q,
        request_settings: RequestSettings,
    ) -> Result<R::Answer, Error>
    where
        R::Response: Clone,
    {
        let requirement = query.protocol_requirement(&self.query_settings())?;
        self.ensure_network_serves(requirement, Some(request_settings))
            .await?;
        // The request is encoded with the version the SDK speaks once the check above has
        // run: asking the network for its version may have raised it.
        let settings = self.query_settings();
        let request: &R = &query.query(&settings)?;
        let prove = settings.prove;
        let platform_version = settings.protocol_version;

        let fetch = move |local_settings: RequestSettings| async move {
            let ExecutionResponse {
                inner: response,
                address,
                retries,
            } = request
                .clone()
                .execute(self, local_settings)
                .await
                .map_err(|error| error.inner_into())?;

            let answer = if prove {
                self.verify_block_answer(request, response).await
            } else {
                request
                    .unproved_answer(response, platform_version)
                    .map_err(Error::from)
            }
            .map_err(|error| ExecutionError {
                inner: error,
                address: Some(address.clone()),
                retries,
            })?;

            Ok(ExecutionResponse {
                inner: answer,
                address,
                retries,
            })
        };

        let retry_settings = self.dapi_client_settings.override_by(request_settings);
        retry(self.address_list(), retry_settings, fetch)
            .await
            .into_inner()
    }

    /// Verify the answer to `request` against the keys of the context provider, fetching
    /// those of quorums it has not cached. A block is what it is at any age, so nothing
    /// here is held to the SDK's freshness tolerances, and nothing is learned from it.
    async fn verify_block_answer<R: BlockRequest>(
        &self,
        request: &R,
        response: R::Response,
    ) -> Result<R::Answer, Error>
    where
        R::Response: Clone,
    {
        let provider = self
            .context_provider()
            .ok_or(drive_proof_verifier::Error::ContextProviderNotSet)?;
        let verify = || request.proved_answer(response.clone(), &provider);

        match verify_fetching_quorum_keys(&provider, request.max_blocks(), verify).await {
            Ok(answer) => Ok(answer),
            Err(KeyFetchFailure::Refused(error)) => Err((*error).into()),
            Err(KeyFetchFailure::Decided(error)) => Err(*error),
        }
    }
}

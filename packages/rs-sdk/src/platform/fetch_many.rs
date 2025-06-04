//! Fetch multiple objects from Platform.
//!
//! This module provides a trait to fetch multiple objects from Platform.
//!
//! ## Traits
//! - `[FetchMany]`: An async trait that fetches multiple items of a specific type from Platform.

use super::LimitQuery;
use crate::platform::documents::document_query::DocumentQuery;
use crate::{error::Error, mock::MockResponse, platform::query::Query, sync::retry, Sdk};
use dapi_grpc::platform::v0::{
    GetContestedResourceIdentityVotesRequest, GetContestedResourceVoteStateRequest,
    GetContestedResourceVotersForIdentityRequest, GetContestedResourcesRequest,
    GetDataContractsRequest, GetEpochsInfoRequest, GetEvonodesProposedEpochBlocksByIdsRequest,
    GetEvonodesProposedEpochBlocksByRangeRequest, GetIdentitiesBalancesRequest,
    GetIdentityKeysRequest, GetPathElementsRequest, GetProtocolVersionUpgradeStateRequest,
    GetProtocolVersionUpgradeVoteStatusRequest, GetTokenDirectPurchasePricesRequest,
    GetVotePollsByEndDateRequest, Proof, ResponseMetadata,
};
use dashcore_rpc::dashcore::ProTxHash;
use dpp::identity::KeyID;
use dpp::prelude::{Identifier, IdentityPublicKey};
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::ProtocolVersionVoteCount;
use dpp::{block::epoch::EpochIndex, prelude::TimestampMillis, voting::vote_polls::VotePoll};
use dpp::{
    block::extended_epoch_info::ExtendedEpochInfo, voting::votes::resource_vote::ResourceVote,
};
use dpp::{data_contract::DataContract, tokens::token_pricing_schedule::TokenPricingSchedule};
use dpp::{document::Document, voting::contender_structs::ContenderWithSerializedDocument};
use drive::grovedb::query_result_type::Key;
use drive::grovedb::Element;
use drive_proof_verifier::types::{
    Contenders, ContestedResource, ContestedResources, DataContracts, Elements, ExtendedEpochInfos,
    IdentityBalances, IdentityPublicKeys, MasternodeProtocolVote, MasternodeProtocolVotes,
    ProposerBlockCountById, ProposerBlockCountByRange, ProposerBlockCounts,
    ProtocolVersionUpgrades, ResourceVotesByIdentity, TokenDirectPurchasePrices,
    VotePollsGroupedByTimestamp, Voter, Voters,
};
use drive_proof_verifier::{types::Documents, FromProof};
use rs_dapi_client::{
    transport::TransportRequest, DapiRequest, ExecutionError, ExecutionResponse, InnerInto,
    IntoInner, RequestSettings,
};

/// Fetch multiple objects from Platform.
///
/// To fetch multiple objects from Platform, you need to define some query (criteria that fetched objects must match)
/// and use [FetchMany::fetch_many()] for your object type.
///
/// You can also use convenience methods:
/// * [FetchMany::fetch_many_by_identifiers()] - to fetch multiple objects by their identifiers,
/// * [FetchMany::fetch_many_with_limit()] - to fetch not more than `limit` objects.
///
/// ## Generic Parameters
///
/// - `K`: The type of the key used to index the object
/// - `O`: The type of returned container (eg. map) that holds the fetched objects
///
/// ## Example
///
/// An example use case is to fetch multiple [Data Contracts](dpp::prelude::DataContract) by their [Identifiers](Identifier).
/// As [`&[Identifier]`] implements [Query] for this type of requests, you need to:
///
/// * define [Identifier]s of data contracts to fetch,
/// * create a query by grouping identifiers in a collection, like a [Vec] or a slice,
/// * call [DataContract::fetch_many()](FetchMany::fetch_many()) with the query and an instance of [Sdk].
///
/// ```rust
/// use dash_sdk::{Sdk, platform::{Query, Identifier, FetchMany, DataContract}};
///
/// # const SOME_IDENTIFIER_1 : [u8; 32] = [1; 32];
/// # const SOME_IDENTIFIER_2 : [u8; 32] = [2; 32];
/// let sdk = Sdk::new_mock();
///
/// let id1 = Identifier::new(SOME_IDENTIFIER_1);
/// let id2 = Identifier::new(SOME_IDENTIFIER_2);
///
/// let query = vec![id1, id2];
///
/// let data_contract = DataContract::fetch_many(&sdk, query);
/// ```
#[async_trait::async_trait]
pub trait FetchMany<K: Ord, O: FromIterator<(K, Option<Self>)>>
where
    Self: Sized,
    O: MockResponse
        + FromProof<
            Self::Request,
            Request = Self::Request,
            Response = <<Self as FetchMany<K, O>>::Request as TransportRequest>::Response,
        > + Send
        + Default,
{
    /// Type of request used to fetch multiple objects from Platform.
    ///
    /// Most likely, one of the types defined in [`dapi_grpc::platform::v0`].
    ///
    /// This type must implement [`TransportRequest`] and [`MockRequest`](crate::mock::MockRequest).
    type Request: TransportRequest
        + Into<<O as FromProof<<Self as FetchMany<K, O>>::Request>>::Request>;

    /// Fetch (or search) multiple objects on the Dash Platform
    ///
    /// [`FetchMany::fetch_many()`] is an asynchronous method that fetches multiple objects from the Dash Platform.
    ///
    /// Note that this method might introduce some predefined limit on the number of objects returned.
    /// If you need to specify the limit yourself, use [FetchMany::fetch_many_with_limit()] or [LimitQuery] instead.
    ///
    /// ## Per-object type documentation
    ///
    /// See documentation of [FetchMany trait implementations](FetchMany#foreign-impls) for each object type
    /// for more details, including list of supported [queries](Query).
    ///
    /// ## Generic Parameters
    ///
    /// - `Q`: The type of [Query] used to generate a request
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `query`: A query parameter implementing [`Query`](crate::platform::query::Query) to specify the data to be retrieved.
    ///
    /// ## Returns
    ///
    /// Returns a `Result` containing either:
    ///
    /// * list of objects matching the [Query] indexed by a key type `K`, where an item can be None of
    /// the object was not found for provided key
    /// *  [`Error`](crate::error::Error).
    ///
    /// Note that behavior when no items are found can be either empty collection or collection containing None values.
    ///
    /// ## Usage
    ///
    /// See `tests/fetch/document.rs` for a full example.
    ///
    /// ## Error Handling
    ///
    /// Any errors encountered during the execution are returned as [`Error`](crate::error::Error) instances.
    async fn fetch_many<Q: Query<<Self as FetchMany<K, O>>::Request>>(
        sdk: &Sdk,
        query: Q,
    ) -> Result<O, Error> {
        Self::fetch_many_with_metadata_and_proof(sdk, query, None)
            .await
            .map(|(objects, _, _)| objects)
    }

    /// Fetch multiple objects from Platform with metadata.
    ///
    /// Fetch objects from Platform that satisfy the provided [Query].
    /// This method allows you to retrieve the metadata associated with the response.
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `query`: A query parameter implementing [`crate::platform::query::Query`] to specify the data to be fetched.
    /// - `settings`: An optional `RequestSettings` to give greater flexibility on the request.
    ///
    /// ## Returns
    ///
    /// Returns a `Result` containing either:
    ///
    /// * A tuple `(O, ResponseMetadata)` where `O` is the collection of fetched objects, and `ResponseMetadata` contains metadata about the response.
    /// * [`Error`](crate::error::Error) when an error occurs.
    ///
    /// ## Error Handling
    ///
    /// Any errors encountered during the execution are returned as [Error] instances.
    async fn fetch_many_with_metadata<Q: Query<<Self as FetchMany<K, O>>::Request>>(
        sdk: &Sdk,
        query: Q,
        settings: Option<RequestSettings>,
    ) -> Result<(O, ResponseMetadata), Error> {
        Self::fetch_many_with_metadata_and_proof(sdk, query, settings)
            .await
            .map(|(objects, metadata, _)| (objects, metadata))
    }

    /// Fetch multiple objects from Platform with metadata and underlying proof.
    ///
    /// Fetch objects from Platform that satisfy the provided [Query].
    /// This method allows you to retrieve the metadata and the underlying proof associated with the response.
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `query`: A query parameter implementing [`crate::platform::query::Query`] to specify the data to be fetched.
    /// - `settings`: An optional `RequestSettings` to give greater flexibility on the request.
    ///
    /// ## Returns
    ///
    /// Returns a `Result` containing either:
    ///
    /// * A tuple `(O, ResponseMetadata, Proof)` where `O` is the collection of fetched objects, `ResponseMetadata` contains metadata about the response, and `Proof` is the underlying proof.
    /// * [`Error`](crate::error::Error) when an error occurs.
    ///
    /// ## Error Handling
    ///
    /// Any errors encountered during the execution are returned as [Error] instances.
    async fn fetch_many_with_metadata_and_proof<Q: Query<<Self as FetchMany<K, O>>::Request>>(
        sdk: &Sdk,
        query: Q,
        settings: Option<RequestSettings>,
    ) -> Result<(O, ResponseMetadata, Proof), Error> {
        let request = &query.query(sdk.prove())?;

        let fut = |settings: RequestSettings| async move {
            let ExecutionResponse {
                address,
                retries,
                inner: response,
            } = request
                .clone()
                .execute(sdk, settings)
                .await
                .map_err(|e| e.inner_into())?;

            let object_type = std::any::type_name::<Self>().to_string();
            tracing::trace!(
                request = ?request,
                response = ?response,
                ?address,
                retries,
                object_type,
                "fetched objects from platform"
            );

            sdk.parse_proof_with_metadata_and_proof::<<Self as FetchMany<K, O>>::Request, O>(
                request.clone(),
                response,
            )
            .await
            .map_err(|e| ExecutionError {
                inner: e,
                address: Some(address.clone()),
                retries,
            })
            .map(|(o, metadata, proof)| ExecutionResponse {
                inner: (o.unwrap_or_default(), metadata, proof),
                retries,
                address: address.clone(),
            })
        };

        let settings = sdk
            .dapi_client_settings
            .override_by(settings.unwrap_or_default());

        retry(sdk.address_list(), settings, fut).await.into_inner()
    }

    /// Fetch multiple objects from Platform by their identifiers.
    ///
    /// Convenience method to fetch multiple objects by their identifiers.
    /// See [FetchMany] and [FetchMany::fetch_many()] for more detailed documentation.
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `identifiers`: A collection of [Identifiers](crate::platform::Identifier) to fetch.
    ///
    /// ## Requirements
    ///
    /// `Vec<Identifier>` must implement [Query] for [Self::Request].
    async fn fetch_by_identifiers<I: IntoIterator<Item = Identifier> + Send>(
        sdk: &Sdk,
        identifiers: I,
    ) -> Result<O, Error>
    where
        Vec<Identifier>: Query<<Self as FetchMany<K, O>>::Request>,
    {
        let ids = identifiers.into_iter().collect::<Vec<Identifier>>();
        Self::fetch_many(sdk, ids).await
    }

    /// Fetch multiple objects from Platform with limit.
    ///
    /// Fetches up to `limit` objects matching the `query`.
    /// See [FetchMany] and [FetchMany::fetch_many()] for more detailed documentation.
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `query`: A query parameter implementing [`Query`](crate::platform::query::Query) to specify the data to be retrieved.
    /// - `limit`: Maximum number of objects to fetch.
    async fn fetch_many_with_limit<Q: Query<<Self as FetchMany<K, O>>::Request>>(
        sdk: &Sdk,
        query: Q,
        limit: u32,
    ) -> Result<O, Error>
    where
        LimitQuery<Q>: Query<<Self as FetchMany<K, O>>::Request>,
    {
        let limit_query = LimitQuery {
            limit: Some(limit),
            query,
            start_info: None,
        };

        Self::fetch_many(sdk, limit_query).await
    }
}

/// Fetch documents from Platform.
///
/// Returns [Documents](dpp::document::Document) indexed by their [Identifier](dpp::prelude::Identifier).
///
/// ## Supported query types
///
/// * [DriveQuery](crate::platform::DriveDocumentQuery) - query that specifies document matching criteria
/// * [DocumentQuery](crate::platform::documents::document_query::DocumentQuery)
#[async_trait::async_trait]
impl FetchMany<Identifier, Documents> for Document {
    // We need to use the DocumentQuery type here because the DocumentQuery
    // type stores full contract, which is missing in the GetDocumentsRequest type.
    // TODO: Refactor to use ContextProvider
    type Request = DocumentQuery;
    async fn fetch_many<Q: Query<<Self as FetchMany<Identifier, Documents>>::Request>>(
        sdk: &Sdk,
        query: Q,
    ) -> Result<Documents, Error> {
        let document_query: &DocumentQuery = &query.query(sdk.prove())?;

        retry(sdk.address_list(), sdk.dapi_client_settings, |settings| async move {
            let request = document_query.clone();

            let ExecutionResponse {
                address,
                retries,
                inner: response
            } = request.execute(sdk, settings).await.map_err(|e| e.inner_into())?;

            tracing::trace!(request=?document_query, response=?response, ?address, retries, "fetch multiple documents");

            // let object: Option<BTreeMap<K,Document>> = sdk
            let documents = sdk
                .parse_proof::<DocumentQuery, Documents>(document_query.clone(), response)
                .await
                .map_err(|e| ExecutionError {
                    inner: e,
                    retries,
                    address: Some(address.clone()),
                })?
                .unwrap_or_default();

            Ok(ExecutionResponse {
                inner: documents,
                retries,
                address,
            })
        })
        .await
        .into_inner()
    }
}

/// Retrieve public keys for a given identity.
///
/// Returns [IdentityPublicKeys](drive_proof_verifier::types::IdentityPublicKeys) indexed by
/// [KeyID](dpp::identity::KeyID).
///
/// ## Supported query types
///
/// * [Identifier] - [Identity](crate::platform::Identity) ID for which to retrieve keys
impl FetchMany<KeyID, IdentityPublicKeys> for IdentityPublicKey {
    type Request = GetIdentityKeysRequest;
}

/// Retrieve epochs.
///
/// Returns [ExtendedEpochInfos](drive_proof_verifier::types::ExtendedEpochInfos).
///
/// ## Supported query types
///
/// * [EpochQuery](super::types::epoch::EpochQuery) - query that specifies epoch matching criteria
/// * [EpochIndex](dpp::block::epoch::EpochIndex) - epoch index of first object to find; will return up to
///   [DEFAULT_EPOCH_QUERY_LIMIT](super::query::DEFAULT_EPOCH_QUERY_LIMIT) objects starting from this index
/// * [`LimitQuery<EpochQuery>`](super::LimitQuery), [`LimitQuery<EpochIndex>`](super::LimitQuery) - limit query
///   that allows to specify maximum number of objects to fetch; see also [FetchMany::fetch_many_with_limit()].
impl FetchMany<EpochIndex, ExtendedEpochInfos> for ExtendedEpochInfo {
    type Request = GetEpochsInfoRequest;
}

/// Fetch information about number of votes for each protocol version upgrade.
///
/// Returns [ProtocolVersionUpgrades](drive_proof_verifier::types::ProtocolVersionUpgrades)
/// indexed by [ProtocolVersion](dpp::util::deserializer::ProtocolVersion).
///
/// ## Supported query types
///
/// It requires no query, so you can use `()` as the query parameter.
///
/// ## Example
///
/// ```rust
/// use dash_sdk::{Sdk, platform::FetchMany, Error};
/// use drive_proof_verifier::types::{ProtocolVersionUpgrades, ProtocolVersionVoteCount};
///
/// # tokio_test::block_on(async {
/// let sdk = Sdk::new_mock();
/// let result: Result<ProtocolVersionUpgrades, Error> = ProtocolVersionVoteCount::fetch_many(&sdk, ()).await;
/// # });
/// ```
impl FetchMany<ProtocolVersion, ProtocolVersionUpgrades> for ProtocolVersionVoteCount {
    type Request = GetProtocolVersionUpgradeStateRequest;
}

/// Fetch information about protocol version upgrade voted by each node.
///
/// Returns list of [MasternodeProtocolVotes](drive_proof_verifier::types::MasternodeProtocolVote)
/// indexed by [ProTxHash](dashcore_rpc::dashcore::ProTxHash). Each item in this list represents
/// node protxhash and its preferred protocol version.
///
/// ## Supported query types
///
/// * [ProTxHash](dashcore_rpc::dashcore::ProTxHash) - proTxHash of first object to find; will return up to
///   [DEFAULT_NODES_VOTING_LIMIT](super::query::DEFAULT_NODES_VOTING_LIMIT) objects
/// * [`Option<ProTxHash>`](dashcore_rpc::dashcore::ProTxHash) - proTxHash that can be and [Option]; if it is `None`,
///   the query will return all objects
/// * [`LimitQuery<ProTxHash>`](super::LimitQuery) - limit query that allows to specify maximum number of objects
///   to fetch; see also [FetchMany::fetch_many_with_limit()].
impl FetchMany<ProTxHash, MasternodeProtocolVotes> for MasternodeProtocolVote {
    type Request = GetProtocolVersionUpgradeVoteStatusRequest;
}

/// Fetch information about the proposed block count by proposers for a given epoch.
///
/// Returns list of [ProposerBlockCounts](drive_proof_verifier::types::ProposerBlockCounts)
/// indexed by [ProTxHash](dashcore_rpc::dashcore::ProTxHash). Each item in this list represents
/// node protxhash and the amount of blocks that were proposed.
///
/// ## Supported query types
///
/// * [ProTxHash](dashcore_rpc::dashcore::ProTxHash) - proTxHash of first object to find; will return up to
///   [DEFAULT_NODES_VOTING_LIMIT](super::query::DEFAULT_NODES_VOTING_LIMIT) objects
/// * [`Option<ProTxHash>`](dashcore_rpc::dashcore::ProTxHash) - proTxHash that can be and [Option]; if it is `None`,
///   the query will return all objects
/// * [`LimitQuery<ProTxHash>`](super::LimitQuery) - limit query that allows to specify maximum number of objects
///   to fetch; see also [FetchMany::fetch_many_with_limit()].
impl FetchMany<ProTxHash, ProposerBlockCounts> for ProposerBlockCountByRange {
    type Request = GetEvonodesProposedEpochBlocksByRangeRequest;
}

/// Fetch information about the proposed block count by proposers for a given epoch.
///
/// Returns list of [ProposerBlockCounts](drive_proof_verifier::types::ProposerBlockCounts)
/// indexed by [ProTxHash](dashcore_rpc::dashcore::ProTxHash). Each item in this list represents
/// node pro_tx_hash and the amount of blocks that were proposed.
///
/// ## Supported query types
///
/// * [ProTxHash](dashcore_rpc::dashcore::ProTxHash) - proTxHash of an evonode to find; will return one evonode block count
impl FetchMany<ProTxHash, ProposerBlockCounts> for ProposerBlockCountById {
    type Request = GetEvonodesProposedEpochBlocksByIdsRequest;
}

/// Fetch multiple data contracts.
///
/// Returns [DataContracts](drive_proof_verifier::types::DataContracts) indexed by [Identifier](dpp::prelude::Identifier).
///
/// ## Supported query types
///
/// * [Vec<Identifier>](dpp::prelude::Identifier) - list of identifiers of data contracts to fetch
///
impl FetchMany<Identifier, DataContracts> for DataContract {
    type Request = GetDataContractsRequest;
}

/// Fetch multiple [ContestedResource], indexed by Identifier.
///
/// ## Supported query types
///
/// * [VotePollsByDocumentTypeQuery]
impl FetchMany<Identifier, ContestedResources> for ContestedResource {
    type Request = GetContestedResourcesRequest;
}

/// Fetch multiple contenders for a contested document resource vote poll.
///
/// Returns [Contender](drive_proof_verifier::types::Contenders) indexed by [Identifier](dpp::prelude::Identifier).
///
/// ## Supported query types
///
/// * [ContestedDocumentVotePollDriveQuery]
#[async_trait::async_trait]
impl FetchMany<Identifier, Contenders> for ContenderWithSerializedDocument {
    type Request = GetContestedResourceVoteStateRequest;
}

///  Fetch voters
/// ## Supported query types
///
/// * [ContestedDocumentVotePollVotesDriveQuery]
impl FetchMany<usize, Voters> for Voter {
    type Request = GetContestedResourceVotersForIdentityRequest;
}

//   GetContestedResourceIdentityVoteStatus
/// Fetch votes of some identity for a contested document resource vote poll.
///
/// ## Supported query types
///
/// * [ContestedResourceVotesGivenByIdentityQuery]
impl FetchMany<Identifier, ResourceVotesByIdentity> for ResourceVote {
    type Request = GetContestedResourceIdentityVotesRequest;
}

//
/// Fetch multiple vote polls grouped by timestamp.
///
/// ## Supported query types
///
/// * [VotePollsByEndDateDriveQuery]
impl FetchMany<TimestampMillis, VotePollsGroupedByTimestamp> for VotePoll {
    type Request = GetVotePollsByEndDateRequest;
}

//
/// Fetch multiple identity balances.
///
/// ## Supported query types
///
/// * [Vec<Identifier>](dpp::prelude::Identifier) - list of identifiers of identities whose balance we want to fetch
impl FetchMany<Identifier, IdentityBalances> for drive_proof_verifier::types::IdentityBalance {
    type Request = GetIdentitiesBalancesRequest;
}

//
/// Fetch multiple elements.
///
/// ## Supported query types
///
/// * [KeysInPath]
impl FetchMany<Key, Elements> for Element {
    type Request = GetPathElementsRequest;
}

/// Fetch multiple prices for token direct purchase.
///
/// ## Supported query types
///
/// * [`&\[Identifier\]`](dpp::prelude::Identifier) - list of identifiers of tokens whose prices we want to fetch
impl FetchMany<Identifier, TokenDirectPurchasePrices> for TokenPricingSchedule {
    type Request = GetTokenDirectPurchasePricesRequest;
}

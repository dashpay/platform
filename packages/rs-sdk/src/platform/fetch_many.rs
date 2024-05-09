//! Fetch multiple objects from the Platform.
//!
//! This module provides a trait to fetch multiple objects from the platform.
//!
//! ## Traits
//! - `[FetchMany]`: An async trait that fetches multiple items of a specific type from the platform.

use crate::{
    error::Error,
    mock::MockResponse,
    platform::{document_query::DocumentQuery, query::Query},
    Sdk,
};
use dapi_grpc::platform::v0::{
    GetDataContractsRequest, GetDocumentsResponse, GetEpochsInfoRequest,
    GetIdentitiesContractKeysRequest, GetIdentityKeysRequest,
    GetProtocolVersionUpgradeStateRequest, GetProtocolVersionUpgradeVoteStatusRequest,
};
use dashcore_rpc::dashcore::ProTxHash;
use dpp::block::epoch::EpochIndex;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::identity::{KeyID, Purpose};
use dpp::prelude::{Identifier, IdentityPublicKey};
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::ProtocolVersionVoteCount;
use drive_proof_verifier::types::{MasternodeProtocolVote, RetrievedObjects};
use drive_proof_verifier::{types::Documents, FromProof};
use rs_dapi_client::{transport::TransportRequest, DapiRequest, RequestSettings};
use std::collections::BTreeMap;

use super::LimitQuery;

/// Fetch multiple objects from the Platform.
///
/// To fetch multiple objects from the platform, you need to define some query (criteria that fetched objects must match)
/// and use [FetchMany::fetch_many()] for your object type.
///
/// You can also use convenience methods:
/// * [FetchMany::fetch_many_by_identifiers()] - to fetch multiple objects by their identifiers,
/// * [FetchMany::fetch_many_with_limit()] - to fetch not more than `limit` objects.
///
/// ## Generic Parameters
///
/// - `K`: The type of the key used to index the object
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
pub trait FetchMany<K: Ord>
where
    Self: Sized,
    BTreeMap<K, Option<Self>>: MockResponse
        + FromProof<
            Self::Request,
            Request = Self::Request,
            Response = <<Self as FetchMany<K>>::Request as TransportRequest>::Response,
        > + Sync,
{
    /// Type of request used to fetch multiple objects from the platform.
    ///
    /// Most likely, one of the types defined in [`dapi_grpc::platform::v0`].
    ///
    /// This type must implement [`TransportRequest`] and [`MockRequest`](crate::mock::MockRequest).
    type Request: TransportRequest
        + Into<<BTreeMap<K, Option<Self>> as FromProof<<Self as FetchMany<K>>::Request>>::Request>;

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
    /// - `K`: The type of the key used to index the object in the returned collection
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `query`: A query parameter implementing [`Query`](crate::platform::query::Query) to specify the data to be retrieved.
    ///
    /// ## Returns
    ///
    /// Returns a `Result` containing either :
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
    async fn fetch_many<Q: Query<<Self as FetchMany<K>>::Request>>(
        sdk: &Sdk,
        query: Q,
    ) -> Result<RetrievedObjects<K, Self>, Error> {
        let request = query.query(sdk.prove())?;

        let response = request
            .clone()
            .execute(sdk, RequestSettings::default())
            .await?;

        let object_type = std::any::type_name::<Self>().to_string();
        tracing::trace!(request = ?request, response = ?response, object_type, "fetched object from platform");

        let object: BTreeMap<K, Option<Self>> = sdk
            .parse_proof::<<Self as FetchMany<K>>::Request, BTreeMap<K, Option<Self>>>(
                request, response,
            )?
            .unwrap_or_default();

        Ok(object)
    }

    /// Fetch multiple objects from the Platform by their identifiers.
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
    ) -> Result<RetrievedObjects<K, Self>, Error>
    where
        Vec<Identifier>: Query<<Self as FetchMany<K>>::Request>,
    {
        let ids = identifiers.into_iter().collect::<Vec<Identifier>>();
        Self::fetch_many(sdk, ids).await
    }

    /// Fetch multiple objects from the Platform with limit.
    ///
    /// Fetches up to `limit` objects matching the `query`.    
    /// See [FetchMany] and [FetchMany::fetch_many()] for more detailed documentation.
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `query`: A query parameter implementing [`Query`](crate::platform::query::Query) to specify the data to be retrieved.
    /// - `limit`: Maximum number of objects to fetch.
    async fn fetch_many_with_limit<Q: Query<<Self as FetchMany<K>>::Request>>(
        sdk: &Sdk,
        query: Q,
        limit: u32,
    ) -> Result<RetrievedObjects<K, Self>, Error>
    where
        LimitQuery<Q>: Query<<Self as FetchMany<K>>::Request>,
    {
        let limit_query = LimitQuery {
            limit: Some(limit),
            query,
        };

        Self::fetch_many(sdk, limit_query).await
    }
}

/// Fetch documents from the Platform.
///
/// Returns [Documents](dpp::document::Document) indexed by their [Identifier](dpp::prelude::Identifier).
///
/// ## Supported query types
///
/// * [DriveQuery](crate::platform::DriveQuery) - query that specifies document matching criteria
/// * [DocumentQuery](crate::platform::document_query::DocumentQuery) -
#[async_trait::async_trait]
impl FetchMany<Identifier> for Document {
    // We need to use the DocumentQuery type here because the DocumentQuery
    // type stores full contract, which is missing in the GetDocumentsRequest type.
    // TODO: Refactor to use ContextProvider
    type Request = DocumentQuery;
    async fn fetch_many<Q: Query<<Self as FetchMany<Identifier>>::Request>>(
        sdk: &Sdk,
        query: Q,
    ) -> Result<BTreeMap<Identifier, Option<Self>>, Error> {
        let document_query: DocumentQuery = query.query(sdk.prove())?;

        let request = document_query.clone();
        let response: GetDocumentsResponse =
            request.execute(sdk, RequestSettings::default()).await?;

        tracing::trace!(request=?document_query, response=?response, "fetch multiple documents");

        // let object: Option<BTreeMap<K,Document>> = sdk
        let documents: BTreeMap<Identifier, Option<Document>> = sdk
            .parse_proof::<DocumentQuery, Documents>(document_query, response)?
            .unwrap_or_default();

        Ok(documents)
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
impl FetchMany<KeyID> for IdentityPublicKey {
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
/// [DEFAULT_EPOCH_QUERY_LIMIT](super::query::DEFAULT_EPOCH_QUERY_LIMIT) objects starting from this index
/// * [`LimitQuery<EpochQuery>`](super::LimitQuery), [`LimitQuery<EpochIndex>`](super::LimitQuery) - limit query
/// that allows to specify maximum number of objects to fetch; see also [FetchMany::fetch_many_with_limit()].
impl FetchMany<EpochIndex> for ExtendedEpochInfo {
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
/// use dash_sdk::{Sdk, platform::FetchMany};
/// use drive_proof_verifier::types::ProtocolVersionVoteCount;
///
/// # tokio_test::block_on(async {
/// let sdk = Sdk::new_mock();
/// let result = ProtocolVersionVoteCount::fetch_many(&sdk, ()).await;
/// # });
/// ```
impl FetchMany<ProtocolVersion> for ProtocolVersionVoteCount {
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
/// [DEFAULT_NODES_VOTING_LIMIT](super::query::DEFAULT_NODES_VOTING_LIMIT) objects
/// * [`Option<ProTxHash>`](dashcore_rpc::dashcore::ProTxHash) - proTxHash that can be and [Option]; if it is `None`,
/// the query will return all objects
/// * [`LimitQuery<ProTxHash>`](super::LimitQuery) - limit query that allows to specify maximum number of objects
/// to fetch; see also [FetchMany::fetch_many_with_limit()].
impl FetchMany<ProTxHash> for MasternodeProtocolVote {
    type Request = GetProtocolVersionUpgradeVoteStatusRequest;
}

/// Fetch multiple data contracts.
///
/// Returns [DataContracts](drive_proof_verifier::types::DataContracts) indexed by [Identifier](dpp::prelude::Identifier).
impl FetchMany<Identifier> for DataContract {
    type Request = GetDataContractsRequest;
}

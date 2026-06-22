//! # Fetch Module
//!
//! This module provides an abstract way to fetch data from a platform using the `Fetch` trait.
//! It allows fetching of various types of data such as `Identity`, `DataContract`, and `Document`.
//!
//! ## Traits
//! - [Fetch]: An asynchronous trait that defines how to fetch data from Platform.
//!   It requires the implementing type to also implement [Debug] and [FromProof]
//!   traits. The associated [`Fetch::Query`] / [`Fetch::Request`] types split the
//!   user-facing query (what `FromProof` consumes) from the wire-encoded proto
//!   (what flows over the network); for non-versioned operations they alias to
//!   the same proto type, for versioned ones (today: documents) they differ.

use crate::mock::MockResponse;
use crate::sync::retry;
use crate::{error::Error, platform::query::Query, Sdk};
use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::v0::{self as platform_proto, Proof, ResponseMetadata};
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::identity::identities_contract_keys::IdentitiesContractKeys;
use dpp::voting::votes::Vote;
use dpp::{
    block::extended_epoch_info::ExtendedEpochInfo, document::Document, platform_value::Identifier,
    prelude::Identity,
};
use drive_proof_verifier::FromProof;
use rs_dapi_client::{transport::TransportRequest, DapiRequest, RequestSettings};
use rs_dapi_client::{ExecutionError, ExecutionResponse, InnerInto, IntoInner};
use std::fmt::Debug;

use super::types::identity::IdentityRequest;
use super::DocumentQuery;

/// Trait implemented by objects that can be fetched from Platform.
///
/// To fetch an object from Platform, you need to define some query (criteria that fetched object must match) and
/// use [Fetch::fetch()] for your object type.
///
/// Implementors of this trait must define the associated [`Fetch::Request`] type.
/// All methods have default implementations; override [fetch_with_metadata_and_proof()](Fetch::fetch_with_metadata_and_proof)
/// if custom fetch logic is needed, as other methods are convenience methods that call it.
///
/// ## Example
///
/// A common use case is to fetch an [Identity] object by its [Identifier]. As [Identifier] implements [Query] for
/// identity requests, you need to:
/// * create a [Query], which will be an [Identifier] instance that will be used to identify requested [Identity],
/// * call [Identity::fetch()] with the query and an instance of [Sdk].
///
/// ```rust
/// use dash_sdk::{Sdk, platform::{Query, Identifier, Fetch, Identity}};
///
/// # const SOME_IDENTIFIER : [u8; 32] = [0; 32];
/// let sdk = Sdk::new_mock();
/// let query = Identifier::new(SOME_IDENTIFIER);
///
/// let identity = Identity::fetch(&sdk, query);
/// ```
#[async_trait::async_trait]
pub trait Fetch
where
    Self: Sized
        + Debug
        + MockResponse
        + FromProof<
            <Self as Fetch>::Query,
            Request = <Self as Fetch>::Query,
            Response = <<Self as Fetch>::Request as DapiRequest>::Response,
        >,
{
    /// User-facing query type — the rich form that callers hand to
    /// the SDK and that [`FromProof`] binds to. For non-versioned
    /// operations this aliases to [`Self::Request`] (the wire proto);
    /// for versioned ones (e.g. documents) it stays the rich pre-wire
    /// form (e.g. [`DocumentQuery`]) so the proof verifier keeps its
    /// context (data contract, document type) without re-fetching.
    //
    // Associated-type defaults are nightly-only (RFC 2532); each impl
    // must spell out `type Query = Self::Request;` when the rich and
    // wire forms coincide.
    type Query: Query<<Self as Fetch>::Request> + Mockable + Clone + Debug + Send + Sync;

    /// Wire-encoded request that hits the network. Implements
    /// [`TransportRequest`]. For non-versioned operations
    /// `Self::Query = Self::Request`; for versioned ones the
    /// [`Query::query`] impl on [`Self::Query`] performs the
    /// protocol-version-aware wire encoding using `&Sdk`.
    type Request: TransportRequest;

    /// Fetch single object from Platform.
    ///
    /// Fetch object from Platform that satisfies provided [Query].
    /// Most often, the Query is an [Identifier] of the object to be fetched.
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `query`: A query parameter implementing [`crate::platform::query::Query`] to specify the data to be fetched.
    ///
    /// ## Returns
    ///
    /// Returns:
    /// * `Ok(Some(Self))` when object is found
    /// * `Ok(None)` when object is not found
    /// * [`Err(Error)`](Error) when an error occurs
    ///
    /// ## Error Handling
    ///
    /// Any errors encountered during the execution are returned as [Error] instances.
    async fn fetch<Q: Query<<Self as Fetch>::Query>>(
        sdk: &Sdk,
        query: Q,
    ) -> Result<Option<Self>, Error> {
        Self::fetch_with_settings(sdk, query, RequestSettings::default()).await
    }

    /// Fetch single object from Platform with metadata.
    ///
    /// Fetch object from Platform that satisfies provided [Query].
    /// Most often, the Query is an [Identifier] of the object to be fetched.
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `query`: A query parameter implementing [`crate::platform::query::Query`] to specify the data to be fetched.
    /// - `settings`: An optional `RequestSettings` to give greater flexibility on the request.
    ///
    /// ## Returns
    ///
    /// Returns a tuple of the fetched object and [ResponseMetadata]:
    /// * `Ok((Some(Self), ResponseMetadata))` when object is found
    /// * `Ok((None, ResponseMetadata))` when object is not found
    /// * [`Err(Error)`](Error) when an error occurs
    ///
    /// ## Error Handling
    ///
    /// Any errors encountered during the execution are returned as [Error] instances.
    async fn fetch_with_metadata<Q: Query<<Self as Fetch>::Query>>(
        sdk: &Sdk,
        query: Q,
        settings: Option<RequestSettings>,
    ) -> Result<(Option<Self>, ResponseMetadata), Error> {
        Self::fetch_with_metadata_and_proof(sdk, query, settings)
            .await
            .map(|(object, metadata, _)| (object, metadata))
    }

    /// Fetch single object from Platform with metadata and underlying proof.
    ///
    /// Fetch object from Platform that satisfies provided [Query].
    /// Most often, the Query is an [Identifier] of the object to be fetched.
    ///
    /// This method is meant to give the library user a way to see the underlying proof
    /// for educational purposes. This method should most likely only be used for debugging.
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `query`: A query parameter implementing [`crate::platform::query::Query`] to specify the data to be fetched.
    /// - `settings`: An optional `RequestSettings` to give greater flexibility on the request.
    ///
    /// ## Returns
    ///
    /// Returns a tuple of the fetched object, [ResponseMetadata], and the underlying [Proof]:
    /// * `Ok((Some(Self), ResponseMetadata, Proof))` when object is found
    /// * `Ok((None, ResponseMetadata, Proof))` when object is not found
    /// * [`Err(Error)`](Error) when an error occurs
    ///
    /// ## Error Handling
    ///
    /// Any errors encountered during the execution are returned as [Error] instances.
    async fn fetch_with_metadata_and_proof<Q: Query<<Self as Fetch>::Query>>(
        sdk: &Sdk,
        query: Q,
        request_settings: Option<RequestSettings>,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error> {
        let settings = sdk.query_settings();
        let owned_rich: <Self as Fetch>::Query = query.query(&settings)?;
        // INTENTIONAL(#3711): For the common case `Self::Query = Self::Request`,
        // the blanket `Query<T> for T` impl turns the `query.query(settings)` step into a
        // pure clone of the same owned request. Real but micro-cost (~63 impls hit
        // this path). Specializing via a `fn encode_request_owned()` default method on
        // `Fetch` would eliminate the clone — deferred as future-perf work; trait shape
        // is intentionally uniform for now.
        let owned_wire: <Self as Fetch>::Request = owned_rich.query(&settings)?;
        let rich = &owned_rich;
        let wire = &owned_wire;

        let fut = |settings: RequestSettings| async move {
            let ExecutionResponse {
                address,
                retries,
                inner: response,
            } = wire
                .clone()
                .execute(sdk, settings)
                .await
                .map_err(|execution_error| execution_error.inner_into())?;

            let object_type = std::any::type_name::<Self>().to_string();
            tracing::trace!(request = ?wire, response = ?response, ?address, retries, object_type, "fetched object from platform");

            let (object, response_metadata, proof): (Option<Self>, ResponseMetadata, Proof) = sdk
                .parse_proof_with_metadata_and_proof::<<Self as Fetch>::Query, Self>(
                    rich.clone(),
                    response,
                    wire.method_name(),
                )
                .await
                .map_err(|e| ExecutionError {
                    inner: e,
                    address: Some(address.clone()),
                    retries,
                })?;

            match object {
                Some(item) => Ok((item.into(), response_metadata, proof)),
                None => Ok((None, response_metadata, proof)),
            }
            .map(|x| ExecutionResponse {
                inner: x,
                address,
                retries,
            })
        };

        let retry_settings = sdk
            .dapi_client_settings
            .override_by(request_settings.unwrap_or_default());

        retry(sdk.address_list(), retry_settings, fut)
            .await
            .into_inner()
    }

    /// Fetch single object from Platform.
    ///
    /// Fetch object from Platform that satisfies provided [Query].
    /// Most often, the Query is an [Identifier] of the object to be fetched.
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `query`: A query parameter implementing [`crate::platform::query::Query`] to specify the data to be fetched.
    /// - `settings`: Request settings for the connection to Platform.
    ///
    /// ## Returns
    ///
    /// Returns:
    /// * `Ok(Some(Self))` when object is found
    /// * `Ok(None)` when object is not found
    /// * [`Err(Error)`](Error) when an error occurs
    ///
    /// ## Error Handling
    ///
    /// Any errors encountered during the execution are returned as [Error] instances.
    async fn fetch_with_settings<Q: Query<<Self as Fetch>::Query>>(
        sdk: &Sdk,
        query: Q,
        settings: RequestSettings,
    ) -> Result<Option<Self>, Error> {
        let (object, _) = Self::fetch_with_metadata(sdk, query, Some(settings)).await?;
        Ok(object)
    }

    /// Fetch single object from Platform by identifier.
    ///
    /// Convenience method that allows fetching objects by identifier for types that implement [Query] for [Identifier].
    ///
    /// See [`Fetch::fetch()`] for more details.
    ///
    /// ## Parameters
    ///
    /// - `sdk`: An instance of [Sdk].
    /// - `id`: An [Identifier] of the object to be fetched.
    async fn fetch_by_identifier(sdk: &Sdk, id: Identifier) -> Result<Option<Self>, Error>
    where
        Identifier: Query<<Self as Fetch>::Query>,
    {
        Self::fetch(sdk, id).await
    }
}

impl Fetch for Identity {
    type Query = IdentityRequest;
    type Request = IdentityRequest;
}

impl Fetch for dpp::prelude::DataContract {
    type Query = platform_proto::GetDataContractRequest;
    type Request = platform_proto::GetDataContractRequest;
}

impl Fetch for (dpp::prelude::DataContract, Vec<u8>) {
    type Query = platform_proto::GetDataContractRequest;
    type Request = platform_proto::GetDataContractRequest;
}

impl Fetch for Document {
    type Query = DocumentQuery;
    type Request = platform_proto::GetDocumentsRequest;
}

impl Fetch for drive_proof_verifier::types::IdentityBalance {
    type Query = platform_proto::GetIdentityBalanceRequest;
    type Request = platform_proto::GetIdentityBalanceRequest;
}

impl Fetch for drive_proof_verifier::types::AddressInfo {
    type Query = platform_proto::GetAddressInfoRequest;
    type Request = platform_proto::GetAddressInfoRequest;
}

impl Fetch for drive_proof_verifier::types::TotalCreditsInPlatform {
    type Query = platform_proto::GetTotalCreditsInPlatformRequest;
    type Request = platform_proto::GetTotalCreditsInPlatformRequest;
}

impl Fetch for drive_proof_verifier::types::IdentityNonceFetcher {
    type Query = platform_proto::GetIdentityNonceRequest;
    type Request = platform_proto::GetIdentityNonceRequest;
}

impl Fetch for drive_proof_verifier::types::IdentityContractNonceFetcher {
    type Query = platform_proto::GetIdentityContractNonceRequest;
    type Request = platform_proto::GetIdentityContractNonceRequest;
}

impl Fetch for drive_proof_verifier::types::IdentityBalanceAndRevision {
    type Query = platform_proto::GetIdentityBalanceAndRevisionRequest;
    type Request = platform_proto::GetIdentityBalanceAndRevisionRequest;
}

impl Fetch for drive_proof_verifier::types::DataContractHistory {
    type Query = platform_proto::GetDataContractHistoryRequest;
    type Request = platform_proto::GetDataContractHistoryRequest;
}

impl Fetch for drive_proof_verifier::types::DocumentHistory {
    type Query = platform_proto::GetDocumentHistoryRequest;
    type Request = platform_proto::GetDocumentHistoryRequest;
}

impl Fetch for ExtendedEpochInfo {
    type Query = platform_proto::GetEpochsInfoRequest;
    type Request = platform_proto::GetEpochsInfoRequest;
}

impl Fetch for drive_proof_verifier::types::PrefundedSpecializedBalance {
    type Query = platform_proto::GetPrefundedSpecializedBalanceRequest;
    type Request = platform_proto::GetPrefundedSpecializedBalanceRequest;
}

impl Fetch for Vote {
    type Query = platform_proto::GetContestedResourceIdentityVotesRequest;
    type Request = platform_proto::GetContestedResourceIdentityVotesRequest;
}

impl Fetch for RewardDistributionMoment {
    type Query = platform_proto::GetTokenPerpetualDistributionLastClaimRequest;
    type Request = platform_proto::GetTokenPerpetualDistributionLastClaimRequest;
}

/// Fetch contract-scoped keys for multiple identities.
impl Fetch for IdentitiesContractKeys {
    type Query = platform_proto::GetIdentitiesContractKeysRequest;
    type Request = platform_proto::GetIdentitiesContractKeysRequest;
}

impl Fetch for dpp::tokens::contract_info::TokenContractInfo {
    type Query = platform_proto::GetTokenContractInfoRequest;
    type Request = platform_proto::GetTokenContractInfoRequest;
}

impl Fetch for drive_proof_verifier::types::RecentAddressBalanceChanges {
    type Query = platform_proto::GetRecentAddressBalanceChangesRequest;
    type Request = platform_proto::GetRecentAddressBalanceChangesRequest;
}

impl Fetch for drive_proof_verifier::types::RecentCompactedAddressBalanceChanges {
    type Query = platform_proto::GetRecentCompactedAddressBalanceChangesRequest;
    type Request = platform_proto::GetRecentCompactedAddressBalanceChangesRequest;
}

impl Fetch for drive_proof_verifier::types::PlatformAddressTrunkState {
    type Query = platform_proto::GetAddressesTrunkStateRequest;
    type Request = platform_proto::GetAddressesTrunkStateRequest;
}

impl Fetch for drive_proof_verifier::types::ShieldedPoolState {
    type Query = platform_proto::GetShieldedPoolStateRequest;
    type Request = platform_proto::GetShieldedPoolStateRequest;
}

impl Fetch for drive_proof_verifier::types::ShieldedNotesCount {
    type Query = platform_proto::GetShieldedNotesCountRequest;
    type Request = platform_proto::GetShieldedNotesCountRequest;
}

impl Fetch for drive_proof_verifier::types::ShieldedAnchors {
    type Query = platform_proto::GetShieldedAnchorsRequest;
    type Request = platform_proto::GetShieldedAnchorsRequest;
}

impl Fetch for drive_proof_verifier::types::MostRecentShieldedAnchor {
    type Query = platform_proto::GetMostRecentShieldedAnchorRequest;
    type Request = platform_proto::GetMostRecentShieldedAnchorRequest;
}

impl Fetch for drive_proof_verifier::types::ShieldedEncryptedNotes {
    type Query = platform_proto::GetShieldedEncryptedNotesRequest;
    type Request = platform_proto::GetShieldedEncryptedNotesRequest;
}

impl Fetch for drive_proof_verifier::types::ShieldedNullifierStatuses {
    type Query = platform_proto::GetShieldedNullifiersRequest;
    type Request = platform_proto::GetShieldedNullifiersRequest;
}

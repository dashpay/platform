//! # Fetch Module
//!
//! This module provides an abstract way to fetch data from a platform using the `Fetch` trait.
//! It allows fetching of various types of data such as `Identity`, `DataContract`, and `Document`.
//!
//! ## Traits
//! - [Fetch]: An asynchronous trait that defines how to fetch data from Platform.
//!   It requires the implementing type to also implement [Debug] and [FromProof]
//!   traits. The associated [`Fetch::Request`] type needs to implement [TransportRequest].

use crate::mock::MockResponse;
use crate::sync::retry;
use crate::{error::Error, platform::query::Query, Sdk};
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
            <Self as Fetch>::Request,
            Request = <Self as Fetch>::Request,
            Response = <<Self as Fetch>::Request as DapiRequest>::Response,
        >,
{
    /// Type of request used to fetch data from Platform.
    ///
    /// Most likely, one of the types defined in [`dapi_grpc::platform::v0`].
    ///
    /// This type must implement [`TransportRequest`].
    type Request: TransportRequest + Into<<Self as FromProof<<Self as Fetch>::Request>>::Request>;

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
    async fn fetch<Q: Query<<Self as Fetch>::Request>>(
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
    async fn fetch_with_metadata<Q: Query<<Self as Fetch>::Request>>(
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
    async fn fetch_with_metadata_and_proof<Q: Query<<Self as Fetch>::Request>>(
        sdk: &Sdk,
        query: Q,
        settings: Option<RequestSettings>,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error> {
        let request: &<Self as Fetch>::Request = &query.query(sdk.prove())?;

        let fut = |settings: RequestSettings| async move {
            let ExecutionResponse {
                address,
                retries,
                inner: response,
            } = request
                .clone()
                .execute(sdk, settings)
                .await
                .map_err(|execution_error| execution_error.inner_into())?;

            let object_type = std::any::type_name::<Self>().to_string();
            tracing::trace!(request = ?request, response = ?response, ?address, retries, object_type, "fetched object from platform");

            let (object, response_metadata, proof): (Option<Self>, ResponseMetadata, Proof) = sdk
                .parse_proof_with_metadata_and_proof(request.clone(), response)
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

        let settings = sdk
            .dapi_client_settings
            .override_by(settings.unwrap_or_default());

        retry(sdk.address_list(), settings, fut).await.into_inner()
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
    async fn fetch_with_settings<Q: Query<<Self as Fetch>::Request>>(
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
        Identifier: Query<<Self as Fetch>::Request>,
    {
        Self::fetch(sdk, id).await
    }
}

impl Fetch for Identity {
    type Request = IdentityRequest;
}

impl Fetch for dpp::prelude::DataContract {
    type Request = platform_proto::GetDataContractRequest;
}

impl Fetch for (dpp::prelude::DataContract, Vec<u8>) {
    type Request = platform_proto::GetDataContractRequest;
}

impl Fetch for Document {
    type Request = DocumentQuery;
}

impl Fetch for drive_proof_verifier::types::IdentityBalance {
    type Request = platform_proto::GetIdentityBalanceRequest;
}

impl Fetch for drive_proof_verifier::types::AddressInfo {
    type Request = platform_proto::GetAddressInfoRequest;
}

impl Fetch for drive_proof_verifier::types::TotalCreditsInPlatform {
    type Request = platform_proto::GetTotalCreditsInPlatformRequest;
}

impl Fetch for drive_proof_verifier::types::IdentityNonceFetcher {
    type Request = platform_proto::GetIdentityNonceRequest;
}

impl Fetch for drive_proof_verifier::types::IdentityContractNonceFetcher {
    type Request = platform_proto::GetIdentityContractNonceRequest;
}

impl Fetch for drive_proof_verifier::types::IdentityBalanceAndRevision {
    type Request = platform_proto::GetIdentityBalanceAndRevisionRequest;
}

impl Fetch for drive_proof_verifier::types::DataContractHistory {
    type Request = platform_proto::GetDataContractHistoryRequest;
}

impl Fetch for ExtendedEpochInfo {
    type Request = platform_proto::GetEpochsInfoRequest;
}

impl Fetch for drive_proof_verifier::types::PrefundedSpecializedBalance {
    type Request = platform_proto::GetPrefundedSpecializedBalanceRequest;
}

impl Fetch for Vote {
    type Request = platform_proto::GetContestedResourceIdentityVotesRequest;
}

impl Fetch for RewardDistributionMoment {
    type Request = platform_proto::GetTokenPerpetualDistributionLastClaimRequest;
}

/// Fetch contract-scoped keys for multiple identities.
impl Fetch for IdentitiesContractKeys {
    type Request = platform_proto::GetIdentitiesContractKeysRequest;
}

impl Fetch for dpp::tokens::contract_info::TokenContractInfo {
    type Request = platform_proto::GetTokenContractInfoRequest;
}

impl Fetch for drive_proof_verifier::types::RecentAddressBalanceChanges {
    type Request = platform_proto::GetRecentAddressBalanceChangesRequest;
}

impl Fetch for drive_proof_verifier::types::RecentCompactedAddressBalanceChanges {
    type Request = platform_proto::GetRecentCompactedAddressBalanceChangesRequest;
}

impl Fetch for drive_proof_verifier::types::PlatformAddressTrunkState {
    type Request = platform_proto::GetAddressesTrunkStateRequest;
}

impl Fetch for drive_proof_verifier::types::ShieldedPoolState {
    type Request = platform_proto::GetShieldedPoolStateRequest;
}

impl Fetch for drive_proof_verifier::types::ShieldedAnchors {
    type Request = platform_proto::GetShieldedAnchorsRequest;
}

impl Fetch for drive_proof_verifier::types::ShieldedEncryptedNotes {
    type Request = platform_proto::GetShieldedEncryptedNotesRequest;
}

impl Fetch for drive_proof_verifier::types::ShieldedNullifierStatuses {
    type Request = platform_proto::GetShieldedNullifiersRequest;
}

impl Fetch for drive_proof_verifier::types::NullifiersTrunkState {
    type Request = platform_proto::GetNullifiersTrunkStateRequest;
}

impl Fetch for drive_proof_verifier::types::RecentNullifierChanges {
    type Request = platform_proto::GetRecentNullifierChangesRequest;
}

impl Fetch for drive_proof_verifier::types::RecentCompactedNullifierChanges {
    type Request = platform_proto::GetRecentCompactedNullifierChangesRequest;
}

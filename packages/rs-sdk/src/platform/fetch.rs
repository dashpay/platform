//! # Fetch Module
//!
//! This module provides an abstract way to fetch data from a platform using the `Fetch` trait.
//! It allows fetching of various types of data such as `Identity`, `DataContract`, and `Document`.
//!
//! ## Traits
//! - [Fetch]: An asynchronous trait that defines how to fetch data from Platform.
//!   It requires the implementing type to also implement [Debug] and [FromProof]
//!   traits. The associated [Fetch::Request]` type needs to implement [TransportRequest].

use crate::mock::MockResponse;
use crate::sync::retry;
use crate::{error::Error, platform::query::Query, Sdk};
use dash_context_provider::ContextProvider;
use dapi_grpc::platform::v0::{self as platform_proto, Proof, ResponseMetadata};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
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
/// Implementators of this trait should implement at least the [fetch_with_metadata()](Fetch::fetch_with_metadata)
/// method, as other methods are convenience methods that call it with default settings.
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
    /// Returns:
    /// * `Ok(Some(Self))` when object is found
    /// * `Ok(None)` when object is not found
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
    /// This method is meant to give the user library a way to see the underlying proof
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
    /// Returns:
    /// * `Ok(Some(Self))` when object is found
    /// * `Ok(None)` when object is not found
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
        let request = query.query(sdk.prove())?;
        fetch_request(sdk, &request, settings).await
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

#[async_trait::async_trait]
impl Fetch for Document {
    type Request = DocumentQuery;

    async fn fetch_with_metadata_and_proof<Q: Query<<Self as Fetch>::Request>>(
        sdk: &Sdk,
        query: Q,
        settings: Option<RequestSettings>,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error> {
        let document_query: DocumentQuery = query.query(sdk.prove())?;

        // First attempt with current (possibly cached) contract
        match fetch_request(sdk, &document_query, settings).await {
            Ok(result) => Ok(result),
            Err(e) if is_document_deserialization_error(&e) => {
                let fresh_query = refetch_contract_for_query(sdk, &document_query).await?;
                fetch_request(sdk, &fresh_query, settings).await
            }
            Err(e) => Err(e),
        }
    }
}

/// Execute a fetch request with node-level retry logic.
///
/// Shared implementation used by both the default [Fetch::fetch_with_metadata_and_proof]
/// and the [Document]-specific override.
async fn fetch_request<O, R>(
    sdk: &Sdk,
    request: &R,
    settings: Option<RequestSettings>,
) -> Result<(Option<O>, ResponseMetadata, Proof), Error>
where
    O: Sized
        + Send
        + Debug
        + MockResponse
        + FromProof<R, Request = R, Response = <R as DapiRequest>::Response>,
    R: TransportRequest + Into<<O as FromProof<R>>::Request> + Clone + Debug,
{
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

        let object_type = std::any::type_name::<O>().to_string();
        tracing::trace!(request = ?request, response = ?response, ?address, retries, object_type, "fetched object from platform");

        let (object, response_metadata, proof): (Option<O>, ResponseMetadata, Proof) = sdk
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

/// Refetch the data contract from the network, update the context provider
/// cache, and return a new [DocumentQuery] with the fresh contract.
///
/// Used by document fetch retry logic when a deserialization error indicates
/// a stale cached contract.
pub(super) async fn refetch_contract_for_query(
    sdk: &Sdk,
    document_query: &DocumentQuery,
) -> Result<DocumentQuery, Error> {
    tracing::debug!(
        contract_id = ?document_query.data_contract.id(),
        "refetching contract for document query after deserialization failure"
    );

    let fresh_contract = dpp::prelude::DataContract::fetch(sdk, document_query.data_contract.id())
        .await?
        .ok_or(Error::MissingDependency(
            "DataContract".to_string(),
            format!(
                "data contract {} not found during refetch",
                document_query.data_contract.id()
            ),
        ))?;

    let fresh_contract = std::sync::Arc::new(fresh_contract);

    // Update the cached contract in the context provider
    if let Some(context_provider) = sdk.context_provider() {
        context_provider.update_data_contract(fresh_contract.clone());
    }

    Ok(document_query.clone_with_contract(fresh_contract))
}

/// Returns true if the error indicates a document deserialization failure
/// that could be caused by a stale/outdated data contract schema.
pub(super) fn is_document_deserialization_error(error: &Error) -> bool {
    use dpp::data_contract::errors::DataContractError;

    matches!(
        error,
        Error::Proof(drive_proof_verifier::Error::ProtocolError(
            dpp::ProtocolError::DataContractError(DataContractError::CorruptedSerialization(_))
        ))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::errors::DataContractError;

    #[test]
    fn test_corrupted_serialization_is_detected() {
        let error = Error::Proof(drive_proof_verifier::Error::ProtocolError(
            dpp::ProtocolError::DataContractError(DataContractError::CorruptedSerialization(
                "test error".to_string(),
            )),
        ));

        assert!(is_document_deserialization_error(&error));
    }

    #[test]
    fn test_other_protocol_error_is_not_detected() {
        let error = Error::Proof(drive_proof_verifier::Error::ProtocolError(
            dpp::ProtocolError::DecodingError("some decoding error".to_string()),
        ));

        assert!(!is_document_deserialization_error(&error));
    }

    #[test]
    fn test_other_proof_error_is_not_detected() {
        let error = Error::Proof(drive_proof_verifier::Error::EmptyVersion);

        assert!(!is_document_deserialization_error(&error));
    }

    #[test]
    fn test_non_proof_error_is_not_detected() {
        let error = Error::Generic("some error".to_string());

        assert!(!is_document_deserialization_error(&error));
    }
}

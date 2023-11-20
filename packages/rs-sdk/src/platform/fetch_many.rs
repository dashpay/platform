//! Fetch multiple objects from the Platform.
//!
//! This module provides a trait to fetch multiple objects from the platform.
//!
//! ## Traits
//! - `[FetchMany]`: An async trait that fetches multiple items of a specific type from the platform.

use crate::mock::MockResponse;
use crate::{
    error::Error,
    platform::{document_query::DocumentQuery, query::Query},
    Sdk,
};
use dapi_grpc::platform::v0::{
    GetDocumentsResponse, GetEpochsInfoRequest, GetIdentityKeysRequest,
    GetProtocolVersionUpgradeStateRequest, GetProtocolVersionUpgradeVoteStatusRequest,
};
use dashcore_rpc::dashcore::ProTxHash;
use dpp::block::epoch::EpochIndex;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::document::Document;
use dpp::identity::KeyID;
use dpp::prelude::{Identifier, IdentityPublicKey};
use dpp::util::deserializer::ProtocolVersion;
use drive_proof_verifier::types::{ProtocolVersionVoteCount, RetrievedObjects};
use drive_proof_verifier::{types::Documents, FromProof};
use rs_dapi_client::{transport::TransportRequest, DapiRequest, RequestSettings};
use std::collections::BTreeMap;

/// Trait implemented by objects that can be listed or searched.
///
/// ## Generic Parameters
///
/// - `K`: The type of the key used to index the object
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

    /// # Fetch (or search) multiple objects on the Dash Platform
    ///
    /// [`FetchMany::fetch_many()`] is an asynchronous method that fetches multiple objects from Dash Platform.
    ///
    /// ## Parameters
    /// - `sdk`: An instance of [Sdk].
    /// - `query`: A query parameter implementing [`Query`](crate::platform::query::Query) to specify the data to be retrieved.
    ///
    /// ## Returns
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
        sdk: &mut Sdk,
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
}

#[async_trait::async_trait]
impl FetchMany<Identifier> for Document {
    // We need to use the DocumentQuery type here because the DocumentQuery
    // type stores full contract, which is missing in the GetDocumentsRequest type.
    type Request = DocumentQuery;
    async fn fetch_many<Q: Query<<Self as FetchMany<Identifier>>::Request>>(
        sdk: &mut Sdk,
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
impl FetchMany<KeyID> for IdentityPublicKey {
    type Request = GetIdentityKeysRequest;
}

/// Retrieve epochs.
///
/// Returns [ExtendedEpochInfos](drive_proof_verifier::types::ExtendedEpochInfos).
impl FetchMany<EpochIndex> for ExtendedEpochInfo {
    type Request = GetEpochsInfoRequest;
}

/// Fetch information about number of votes for each protocol version upgrade.
///
/// Returns [ProtocolVersionUpgrades](drive_proof_verifier::types::ProtocolVersionUpgrades)
/// indexed by [ProtocolVersion](dpp::util::deserializer::ProtocolVersion).
impl FetchMany<ProtocolVersion> for ProtocolVersionVoteCount {
    type Request = GetProtocolVersionUpgradeStateRequest;
}

/// Fetch information about protocol version upgrade voted by each node.
///
/// Returns [ProtocolVersionVotes](drive_proof_verifier::types::ProtocolVersionVotes)
/// indexed by [ProTxHash](dashcore_rpc::dashcore::ProTxHash).
impl FetchMany<ProTxHash> for ProtocolVersion {
    type Request = GetProtocolVersionUpgradeVoteStatusRequest;
}

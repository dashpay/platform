//! # Fetch Module
//!
//! This module provides an abstract way to fetch data from a platform using the `Fetch` trait.
//! It allows fetching of various types of data such as `Identity`, `DataContract`, and `Document`.
//!
//! ## Traits
//! - [Fetch]: An asynchronous trait that defines how to fetch data from the platform.
//!   It requires the implementing type to also implement [Debug] and [FromProof]
//!   traits. The associated [Fetch::Request]` type needs to implement [TransportRequest].

use crate::mock::{MockRequest, MockResponse};
use crate::{error::Error, platform::query::Query, Sdk};
use dapi_grpc::platform::v0::{self as platform_proto};
use dpp::{document::Document, prelude::Identity};
use drive_proof_verifier::FromProof;
use rs_dapi_client::{transport::TransportRequest, DapiRequest, RequestSettings};
use std::fmt::Debug;

use super::identity::IdentityRequest;
use super::DocumentQuery;

/// Trait implemented by objects that can be fetched from the platform.
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
    /// Type of request used to fetch data from the platform.
    ///
    /// Most likely, one of the types defined in [`dapi_grpc::platform::v0`].
    ///
    /// This type must implement [`TransportRequest`] and [`MockRequest`].
    type Request: TransportRequest
        + MockRequest
        + Into<<Self as FromProof<<Self as Fetch>::Request>>::Request>;

    /// Fetch single object from the Platfom.
    ///
    /// An asynchronous method provided by the Fetch trait that fetches data from Dash Platform.
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
        sdk: &mut Sdk,
        query: Q,
    ) -> Result<Option<Self>, Error> {
        let request = query.query(sdk.prove())?;

        let response = request
            .clone()
            .execute(sdk, RequestSettings::default())
            .await?;

        let object_type = std::any::type_name::<Self>().to_string();
        tracing::trace!(request = ?request, response = ?response, object_type, "fetched object from platform");

        let object: Option<Self> = sdk.parse_proof(request, response)?;

        match object {
            Some(item) => Ok(item.into()),
            None => Ok(None),
        }
    }
}

#[async_trait::async_trait]
impl Fetch for Identity {
    type Request = IdentityRequest;
}

#[async_trait::async_trait]
impl Fetch for dpp::prelude::DataContract {
    type Request = platform_proto::GetDataContractRequest;
}

#[async_trait::async_trait]
impl Fetch for Document {
    type Request = DocumentQuery;
}

#[async_trait::async_trait]
impl Fetch for drive_proof_verifier::types::IdentityBalance {
    type Request = platform_proto::GetIdentityBalanceRequest;
}

#[async_trait::async_trait]
impl Fetch for drive_proof_verifier::types::IdentityBalanceAndRevision {
    type Request = platform_proto::GetIdentityBalanceAndRevisionRequest;
}

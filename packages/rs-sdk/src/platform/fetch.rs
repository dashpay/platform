//! # Fetch Module
//!
//! This module provides an abstract way to fetch data from a platform using the `Fetch` trait.
//! It is designed to be used with any type that implements the `Sdk` trait, and allows for
//! fetching of various types of data such as `Identity`, `DataContract`, and `Document`.
//!
//! ## Traits
//! - [Fetch]: An asynchronous trait that defines how to fetch data from a platform.
//!   It requires the implementing type to also implement [Debug] and [FromProof]
//!   traits. The associated [Fetch::Request]` type needs to implement [TransportRequest].
//!
//! ## Implementations
//! - `Fetch<API>` for [`dpp::prelude::Identity`]
//! - `Fetch<API>` for [`dpp::prelude::DataContract`]
//! - `Fetch<API>` for [`Document`](dpp::document::Document)

use std::fmt::Debug;

use crate::{
    error::Error,
    mock::{MockRequest, MockResponse},
    platform::query::Query,
    Sdk,
};
use dapi_grpc::platform::v0::{self as platform_proto};
use dpp::{document::Document, prelude::Identity};
use drive_proof_verifier::FromProof;
use rs_dapi_client::{transport::TransportRequest, DapiRequest, RequestSettings};

use super::document_query::DocumentQuery;

/// # Fetch Trait
///
/// This trait provides an interface for fetching data from a platform.
///
/// ## Requirements
/// Types that implement this trait must also implement `Debug` and `FromProof`.
/// The associated `Request` type needs to implement `TransportRequest`.
///
/// ## Associated Types
/// - `Request`: The type of request used to fetch data from the platform.
///
/// ## Methods
/// - `fetch`: An asynchronous method that fetches data from a platform. It takes in
///   an instance of `API` (which should implement `Sdk`) and a query parameter,
///   and returns a `Result` containing either the fetched object or an error.
///
/// ## Implementations
/// This trait is implemented by the types `dpp::prelude::Identity`, `dpp::prelude::DataContract`,
/// and `Document`.
///
/// ## Error Handling
/// Any errors encountered during the execution of a fetch operation are returned as `Error` instances.
///
/// ## Extensibility
/// The types for which `Fetch` can be implemented can be easily extended by creating new implementations
/// of `Fetch` for other types.
///
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
    /// Most likely, one of the types defined in `dapi_grpc::platform::v0`.
    ///
    /// This type must implement `TransportRequest`.
    type Request: TransportRequest
        + MockRequest
        + Into<<Self as FromProof<<Self as Fetch>::Request>>::Request>;

    /// Fetch object from the Platfom.
    ///
    /// An asynchronous method provided by the Fetch trait that fetches data from Dash Platform.
    ///
    /// ## Parameters
    /// - `api`: An instance of API which should implement the Sdk trait, most likely
    /// [`DashPlatformSdk`](crate::sdk::DashPlatformSdk).
    /// - `query`: A query parameter implementing [`crate::platform::query::Query`] to specify the data to be fetched.
    ///
    /// ## Returns
    ///
    /// Returns:
    /// * Ok(Some(Self)) when object is found
    /// * Ok(None) when object is not found
    /// * Err(Error) when an error occurs
    ///
    /// ## Error Handling
    /// Any errors encountered during the execution are returned as [Error] instances.
    async fn fetch<Q: Query<<Self as Fetch>::Request>>(
        sdk: &mut Sdk,
        query: Q,
    ) -> Result<Option<Self>, Error> {
        let request = query.query()?;

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
    type Request = platform_proto::GetIdentityRequest;
}

#[async_trait::async_trait]
impl Fetch for dpp::prelude::DataContract {
    type Request = platform_proto::GetDataContractRequest;
}

#[async_trait::async_trait]
impl Fetch for Document {
    type Request = DocumentQuery;
}

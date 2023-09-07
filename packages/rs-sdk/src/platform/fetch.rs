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

use crate::{error::Error, platform::query::Query, Sdk};
use dapi_grpc::platform::v0::{self as platform_proto};
use dpp::document::Document;
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
/// ## Example
/// ```rust
/// let identity = dpp::prelude::Identity::fetch(&api, query).await?;
/// ```
///
/// ## Error Handling
/// Any errors encountered during the execution of a fetch operation are returned as `Error` instances.
///
/// ## Extensibility
/// The types for which `Fetch` can be implemented can be easily extended by creating new implementations
/// of `Fetch` for other types.
///
#[async_trait::async_trait]
pub trait Fetch<API: Sdk>
where
    Self: Sized + Debug + FromProof<Self::Request>,
    <Self as FromProof<Self::Request>>::Response:
        From<<Self::Request as TransportRequest>::Response>,
{
    /// Type of request used to fetch data from the platform.
    ///
    /// Most likely, one of the types defined in `dapi_grpc::platform::v0`.
    ///
    /// This type must implement `TransportRequest`.
    type Request: TransportRequest;

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
    /// Returns a `Result` containing either the fetched object or an error.
    ///
    /// ## Usage
    /// ```rust
    /// let identity = dpp::prelude::Identity::fetch(&api, query).await?;
    /// ```
    ///
    /// ## Error Handling
    /// Any errors encountered during the execution are returned as [Error] instances.
    async fn fetch<Q: Query<Self::Request>>(api: &API, query: Q) -> Result<Option<Self>, Error> {
        let request = query.query()?;

        let mut client = api.platform_client().await;
        let response = request
            .clone()
            .execute(&mut client, RequestSettings::default())
            .await?;

        let object_type = std::any::type_name::<Self>().to_string();
        tracing::trace!(request = ?request, response = ?response, object_type, "fetched object from platform");
        let response = response.into();

        let object = Self::maybe_from_proof(&request, &response, api.quorum_info_provider()?)?;

        match object {
            Some(item) => Ok(item.into()),
            None => Ok(None),
        }
    }
}

#[async_trait::async_trait]
impl<API: Sdk> Fetch<API> for dpp::prelude::Identity {
    type Request = platform_proto::GetIdentityRequest;
}

#[async_trait::async_trait]
impl<API: Sdk> Fetch<API> for dpp::prelude::DataContract {
    type Request = platform_proto::GetDataContractRequest;
}

#[async_trait::async_trait]
impl<API: Sdk> Fetch<API> for Document {
    type Request = DocumentQuery;
}

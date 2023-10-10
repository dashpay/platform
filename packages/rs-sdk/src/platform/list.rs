//! List Module
//!
//! This module provides a trait to fetch a list of objects from the platform.
//!
//! ## Traits
//! - `List`: An async trait that lists items of a specific type from the platform.
//! - `Document`: An implementation of the `List` trait for documents.
//!
//! ## Usage
//! ```rust
//! let documents = dpp::prelude::Document::list(&api, query).await?;
//! ```
//!
//! ## Error Handling
//! Any errors encountered during the execution are returned as Error instances.
use std::{fmt::Debug, ops::DerefMut};

use dapi_grpc::platform::v0::{GetDocumentsRequest, GetDocumentsResponse};
use dpp::document::Document;
use drive::query::DriveQuery;
use drive_proof_verifier::proof::from_proof::{Documents, FromProof};
use rs_dapi_client::{transport::TransportRequest, DapiRequest, RequestSettings};

use crate::{
    error::Error,
    platform::{document_query::DocumentQuery, query::Query},
    Sdk,
};

/// Trait implemented by objects that can be listed or searched.
#[async_trait::async_trait]
pub trait List<API: Sdk>
where
    Self: Sized + Debug,
    Vec<Self>: FromProof<
            Self::Request,
            Response = <<Self as List<API>>::Request as TransportRequest>::Response,
        > + Sync,
{
    /// Type of request used to fetch data from the platform.
    ///
    /// Most likely, one of the types defined in `dapi_grpc::platform::v0`.
    ///
    /// This type must implement `TransportRequest`.
    type Request: TransportRequest + Into<<Vec<Self> as FromProof<Self::Request>>::Request>;

    /// # List or Search for Multiple Objects on the Platform
    ///
    /// `list` is an asynchronous method provided by the `List` trait that fetches multiple objects from Dash Platform.
    ///
    /// ## Parameters
    /// - `api`: An instance of API which should implement the `Sdk` trait. The most common implementation is [`DashPlatformSdk`](crate::sdk::DashPlatformSdk).
    /// - `query`: A query parameter implementing [`Query`](crate::platform::query::Query) to specify the data to be fetched.
    ///
    /// ## Returns
    /// Returns a `Result` containing either a `Option<Vec<Self>>` where `Self` is the type of the fetched object (like a [Document]), or an [`Error`](crate::error::Error).
    ///
    /// ## Usage
    ///
    /// ```rust
    /// let documents = dpp::prelude::Document::list(&api, query).await?;
    /// ```
    ///
    /// See `examples/list_documents.rs` for a full example.
    ///
    /// ## Error Handling
    /// Any errors encountered during the execution are returned as [`Error`](crate::error::Error) instances.
    async fn list<Q: Query<Self::Request>>(
        api: &API,
        query: Q,
    ) -> Result<Option<Vec<Self>>, Error> {
        let request = query.query()?;

        let mut client = api.platform_client().await;
        let response = request
            .clone()
            .execute(client.deref_mut(), RequestSettings::default())
            .await?;

        let object_type = std::any::type_name::<Self>().to_string();
        tracing::trace!(request = ?request, response = ?response, object_type, "fetched object from platform");

        let response: <Vec<Self> as FromProof<<Self as List<API>>::Request>>::Response =
            response.into();
        let object = <Vec<Self> as FromProof<Self::Request>>::maybe_from_proof(
            request,
            response,
            api.quorum_info_provider()?,
        )?;

        match object {
            Some(items) => Ok(items.into()),
            None => Ok(None),
        }
    }
}

#[async_trait::async_trait]
impl<API: Sdk> List<API> for Document {
    // We need to use the DocumentQuery type here because the DocumentQuery
    // type stores full contract, which is missing in the GetDocumentsRequest type.
    type Request = DocumentQuery;
    async fn list<Q: Query<Self::Request>>(
        api: &API,
        query: Q,
    ) -> Result<Option<Vec<Self>>, Error> {
        let document_query: DocumentQuery = query.query()?;
        // We need DriveQuery to verify the proof, as FromProof is implemented for DriveQuery.
        let drive_query: DriveQuery = (&document_query).try_into()?;
        let request: GetDocumentsRequest = document_query.clone().try_into()?;

        let mut client = api.platform_client().await;
        let response: GetDocumentsResponse = request
            .execute(client.deref_mut(), RequestSettings::default())
            .await?;

        tracing::trace!(request=?document_query, response=?response, "list documents");

        match <Documents as FromProof<Self::Request>>::maybe_from_proof(
            &drive_query,
            response,
            api.quorum_info_provider()?,
        )? {
            Some(documents) => Ok(Some(documents)),
            None => Ok(None),
        }
    }
}

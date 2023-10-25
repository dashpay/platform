//! Fetch multiple objects from the Platform.
//!
//! This module provides a trait to fetch multiple objects from the platform.
//!
//! ## Traits
//! - `[FetchMany]`: An async trait that fetches multiple items of a specific type from the platform.

use dapi_grpc::platform::v0::GetDocumentsResponse;
use dpp::document::Document;
use drive_proof_verifier::FromProof;
use rs_dapi_client::{transport::TransportRequest, DapiRequest, RequestSettings};

use crate::mock::MockResponse;
use crate::{
    error::Error,
    platform::{document_query::DocumentQuery, query::Query},
    Sdk,
};
/// Trait implemented by objects that can be listed or searched.
#[async_trait::async_trait]
pub trait FetchMany
where
    Self: Sized,
    Vec<Self>: MockResponse
        + FromProof<
            Self::Request,
            Request = Self::Request,
            Response = <<Self as FetchMany>::Request as TransportRequest>::Response,
        > + Sync,
{
    /// Type of request used to fetch multiple objects from the platform.
    ///
    /// Most likely, one of the types defined in [`dapi_grpc::platform::v0`].
    ///
    /// This type must implement [`TransportRequest`] and [`MockRequest`].
    type Request: TransportRequest
        + Into<<Vec<Self> as FromProof<<Self as FetchMany>::Request>>::Request>;

    /// # Fetch (or search) multiple objects on the Dash Platform
    ///
    /// [`fetch_many()`] is an asynchronous method that fetches multiple objects from Dash Platform.
    ///
    /// ## Parameters
    /// - `sdk`: An instance of [Sdk].
    /// - `query`: A query parameter implementing [`Query`](crate::platform::query::Query) to specify the data to be retrieved.
    ///
    /// ## Returns
    /// Returns a `Result` containing either :
    /// * `Option<Vec<Self>>` where `Self` is the type of the fetched object (like a [Document]), or
    /// *  [`Error`](crate::error::Error).
    ///
    /// ## Usage
    ///
    /// See `tests/fetch/document.rs` for a full example.
    ///
    /// ## Error Handling
    /// Any errors encountered during the execution are returned as [`Error`](crate::error::Error) instances.
    async fn fetch_many<Q: Query<<Self as FetchMany>::Request>>(
        sdk: &mut Sdk,
        query: Q,
    ) -> Result<Option<Vec<Self>>, Error> {
        let request = query.query(sdk.prove())?;

        let response = request
            .clone()
            .execute(sdk, RequestSettings::default())
            .await?;

        let object_type = std::any::type_name::<Self>().to_string();
        tracing::trace!(request = ?request, response = ?response, object_type, "fetched object from platform");

        let object: Option<Vec<Self>> = sdk.parse_proof(request, response)?;

        match object {
            Some(items) => Ok(items.into()),
            None => Ok(None),
        }
    }
}

#[async_trait::async_trait]
impl FetchMany for Document {
    // We need to use the DocumentQuery type here because the DocumentQuery
    // type stores full contract, which is missing in the GetDocumentsRequest type.
    type Request = DocumentQuery;
    async fn fetch_many<Q: Query<<Self as FetchMany>::Request>>(
        sdk: &mut Sdk,
        query: Q,
    ) -> Result<Option<Vec<Self>>, Error> {
        let document_query: DocumentQuery = query.query(sdk.prove())?;

        let request = document_query.clone();
        let response: GetDocumentsResponse =
            request.execute(sdk, RequestSettings::default()).await?;

        tracing::trace!(request=?document_query, response=?response, "fetch multiple documents");

        let object: Option<Vec<Document>> =
            sdk.parse_proof::<DocumentQuery, Vec<Document>>(document_query, response)?;

        match object {
            Some(documents) => Ok(Some(documents)),
            None => Ok(None),
        }
    }
}

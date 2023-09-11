//! Platform DAPI requests.

// TODO: rs-sdk should define a proper user-facing API hiding transport and serialization
// details, however we don't have any data model yet, so for now it will re-export proto
// generated types. Later these re-exports could be swapped with actual rs-sdk's requests
// and while it will change the substance, the API structure will remain the same.

mod document_query;
mod fetch;
mod list;
mod query;

/// Client for the Platform API.
pub type PlatformClient = rs_dapi_client::DapiClient;

pub use dapi_grpc::platform::v0::{self as proto};
pub use dpp::document::Document;
pub use dpp::prelude::{
    DataContract, Identifier, Identity, IdentityPublicKey, Revision, TimestampMillis,
};
pub use {document_query::DocumentQuery, fetch::Fetch, list::List, query::Query};

// use crate::{error::Error, sdk::Sdk};
// TODO this will change, not tested at all
// #[async_trait::async_trait]
// pub trait Modify<A: Sdk, W>
// where
//     Self: Sized,
// {
//     async fn create(self, api: &A, wallet: &W) -> Result<Self, Error>;
//     async fn update(self, api: &A, wallet: &W) -> Result<Self, Error>;
//     async fn delete(self, api: &A, wallet: &W) -> Result<(), Error>;
// }

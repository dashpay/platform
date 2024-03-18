//! Platform DAPI requests.

// TODO: dash-platform-sdk should define a proper user-facing API hiding transport and serialization
// details, however we don't have any data model yet, so for now it will re-export proto
// generated types. Later these re-exports could be swapped with actual dash-platform-sdk's requests
// and while it will change the substance, the API structure will remain the same.

pub mod block_info_from_metadata;
mod delegate;
mod document_query;
mod fetch;
mod fetch_many;
mod query;
pub mod transition;
pub mod types;

pub use dapi_grpc::platform::v0::{self as proto};
pub use dpp::{
    self as dpp,
    document::Document,
    prelude::{DataContract, Identifier, Identity, IdentityPublicKey, Revision},
};
pub use drive::query::DriveQuery;
pub use drive_proof_verifier::ContextProvider;
#[cfg(feature = "mocks")]
pub use drive_proof_verifier::MockContextProvider;
pub use rs_dapi_client as dapi;
pub use {
    document_query::DocumentQuery,
    fetch::Fetch,
    fetch_many::FetchMany,
    query::{LimitQuery, Query, DEFAULT_EPOCH_QUERY_LIMIT},
};

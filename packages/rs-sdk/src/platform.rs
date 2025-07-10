//! Platform DAPI requests.

// TODO: dash-platform-sdk should define a proper user-facing API hiding transport and serialization
// details, however we don't have any data model yet, so for now it will re-export proto
// generated types. Later these re-exports could be swapped with actual dash-platform-sdk's requests
// and while it will change the substance, the API structure will remain the same.

pub mod block_info_from_metadata;
mod delegate;
mod fetch;
pub mod fetch_current_no_parameters;
mod fetch_many;
mod fetch_unproved;
mod identities_contract_keys_query;
pub mod query;
pub mod transition;
pub mod types;

pub mod documents;
pub mod group_actions;
pub mod tokens;

pub use dapi_grpc::platform::v0 as proto;
pub use dash_context_provider::ContextProvider;
#[cfg(feature = "mocks")]
pub use dash_context_provider::MockContextProvider;
pub use documents::document_query::DocumentQuery;
pub use dpp::{
    self as dpp,
    document::Document,
    prelude::{DataContract, Identifier, Identity, IdentityPublicKey, Revision},
};
pub use drive::query::DriveDocumentQuery;
pub use rs_dapi_client as dapi;
pub use {
    fetch::Fetch,
    fetch_many::FetchMany,
    fetch_unproved::FetchUnproved,
    query::{LimitQuery, Query, QueryStartInfo, DEFAULT_EPOCH_QUERY_LIMIT},
};

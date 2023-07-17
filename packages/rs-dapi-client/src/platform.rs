//! Platform DAPI requests.

mod get_documents;
mod get_identity;
mod get_data_contract;

pub use get_identity::{GetIdentity, GetIdentityProof};
pub use get_documents::{GetDocuments, GetDocumentsProof};
pub use get_data_contract::{GetDataContract, GetDataContractProof};

/// Error indicates that the transport response contains insufficient information.
#[derive(Debug, thiserror::Error)]
#[error("returned message contains unexpected `None`s")]
pub struct IncompleteMessage;

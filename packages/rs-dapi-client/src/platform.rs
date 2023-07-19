//! Platform DAPI requests.

mod get_consensus_params;
mod get_data_contract;
mod get_data_contract_history;
mod get_documents;
mod get_identity;

pub use get_consensus_params::GetConsensusParams;
pub use get_data_contract::{GetDataContract, GetDataContractProof};
pub use get_data_contract_history::{GetDataContractHistory, GetDataContractHistoryProof};
pub use get_documents::{GetDocuments, GetDocumentsProof};
pub use get_identity::{GetIdentity, GetIdentityProof};

/// Error indicates that the transport response contains insufficient information.
#[derive(Debug, thiserror::Error)]
#[error("returned message contains unexpected `None`s")]
pub struct IncompleteMessage;

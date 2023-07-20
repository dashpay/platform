//! Platform DAPI requests.

mod broadcast_state_transition;
mod get_consensus_params;
mod get_data_contract;
mod get_data_contract_history;
mod get_documents;
mod get_identities_by_public_key_hashes;
mod get_identity;
mod wait_for_state_transition_result;

pub use broadcast_state_transition::BroadcastStateTransition;
pub use get_consensus_params::GetConsensusParams;
pub use get_data_contract::{GetDataContract, GetDataContractProof};
pub use get_data_contract_history::{GetDataContractHistory, GetDataContractHistoryProof};
pub use get_documents::{GetDocuments, GetDocumentsProof};
pub use get_identities_by_public_key_hashes::GetIdentitiesByPublicKeyHashes;
pub use get_identity::{GetIdentity, GetIdentityProof};
pub use wait_for_state_transition_result::WaitForStateTransitionResult;

/// Error indicates that the transport response contains insufficient information.
#[derive(Debug, thiserror::Error)]
#[error("returned message contains unexpected `None`s")]
pub struct IncompleteMessage;

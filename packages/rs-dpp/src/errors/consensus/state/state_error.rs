use thiserror::Error;

use crate::consensus::state::data_trigger::data_trigger_error::DataTriggerError;
use crate::consensus::state::identity::{
    IdentityAlreadyExistsError, IdentityInsufficientBalanceError,
};
use crate::consensus::ConsensusError;
use crate::{identity::KeyID};
use crate::consensus::state::document::data_contract_already_present_error::DataContractAlreadyPresentError;
use crate::consensus::state::document::document_already_present_error::DocumentAlreadyPresentError;
use crate::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use crate::consensus::state::document::document_owner_id_mismatch_error::DocumentOwnerIdMismatchError;
use crate::consensus::state::document::document_timestamp_window_violation_error::DocumentTimestampWindowViolationError;
use crate::consensus::state::document::document_timestamps_mismatch_error::DocumentTimestampsMismatchError;
use crate::consensus::state::document::duplicate_unique_index_error::DuplicateUniqueIndexError;
use crate::consensus::state::document::invalid_document_revision_error::InvalidDocumentRevisionError;
use crate::consensus::state::identity::duplicated_identity_public_key_id_state_error::DuplicatedIdentityPublicKeyIdStateError;
use crate::consensus::state::identity::duplicated_identity_public_key_state_error::DuplicatedIdentityPublicKeyStateError;
use crate::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;

#[derive(Error, Debug)]
pub enum StateError {
    #[error(transparent)]
    IdentityAlreadyExistsError(IdentityAlreadyExistsError),

    #[error(transparent)]
    IdentityInsufficientBalanceError(IdentityInsufficientBalanceError),

    // Document Errors
    #[error(transparent)]
    DocumentAlreadyPresentError(DocumentAlreadyPresentError),

    #[error(transparent)]
    DocumentNotFoundError(DocumentNotFoundError),

    #[error(transparent)]
    DocumentOwnerIdMismatchError(DocumentOwnerIdMismatchError),

    #[error(transparent)]
    DocumentTimestampsMismatchError(DocumentTimestampsMismatchError),

    #[error(transparent)]
    DocumentTimestampWindowViolationError(DocumentTimestampWindowViolationError),

    #[error(transparent)]
    DuplicateUniqueIndexError(DuplicateUniqueIndexError),

    #[error(transparent)]
    InvalidDocumentRevisionError(InvalidDocumentRevisionError),

    #[error(transparent)]
    DataContractAlreadyPresentError(DataContractAlreadyPresentError),

    #[error(transparent)]
    InvalidIdentityRevisionError(InvalidIdentityRevisionError),

    #[error(transparent)]
    DuplicatedIdentityPublicKeyStateError(DuplicatedIdentityPublicKeyStateError),

    #[error(transparent)]
    DuplicatedIdentityPublicKeyIdStateError(DuplicatedIdentityPublicKeyIdStateError),

    #[error("Identity public keys disabled time ({disabled_at}) is out of block time window from {time_window_start} and {time_window_end}" )]
    IdentityPublicKeyDisabledAtWindowViolationError {
        disabled_at: u64,
        time_window_start: u64,
        time_window_end: u64,
    },

    #[error("Identity Public Key #{public_key_index} is read only")]
    IdentityPublicKeyIsReadOnlyError { public_key_index: KeyID },

    #[error("Identity Public Key with Id {id} does not exist")]
    InvalidIdentityPublicKeyIdError { id: KeyID },

    #[error("Identity cannot contain more than {max_items} public keys")]
    MaxIdentityPublicKeyLimitReachedError { max_items: usize },

    #[error("Identity Public Key #{public_key_index} is disabled")]
    IdentityPublicKeyIsDisabledError { public_key_index: KeyID },

    #[error(transparent)]
    DataTriggerError(DataTriggerError),
}

impl From<StateError> for ConsensusError {
    fn from(error: StateError) -> Self {
        Self::StateError(error)
    }
}

impl From<IdentityAlreadyExistsError> for StateError {
    fn from(err: IdentityAlreadyExistsError) -> Self {
        Self::IdentityAlreadyExistsError(err)
    }
}

impl From<IdentityInsufficientBalanceError> for StateError {
    fn from(err: IdentityInsufficientBalanceError) -> Self {
        Self::IdentityInsufficientBalanceError(err)
    }
}

use thiserror::Error;

use crate::consensus::state::data_contract::data_contract_already_present_error::DataContractAlreadyPresentError;
use crate::consensus::state::data_contract::data_trigger::data_trigger_error::DataTriggerError;
use crate::consensus::state::document::document_already_present_error::DocumentAlreadyPresentError;
use crate::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use crate::consensus::state::document::document_owner_id_mismatch_error::DocumentOwnerIdMismatchError;
use crate::consensus::state::document::document_timestamp_window_violation_error::DocumentTimestampWindowViolationError;
use crate::consensus::state::document::document_timestamps_mismatch_error::DocumentTimestampsMismatchError;
use crate::consensus::state::document::duplicate_unique_index_error::DuplicateUniqueIndexError;
use crate::consensus::state::document::invalid_document_revision_error::InvalidDocumentRevisionError;
use crate::consensus::state::identity::duplicated_identity_public_key_id_state_error::DuplicatedIdentityPublicKeyIdStateError;
use crate::consensus::state::identity::duplicated_identity_public_key_state_error::DuplicatedIdentityPublicKeyStateError;
use crate::consensus::state::identity::identity_public_key_disabled_at_window_violation_error::IdentityPublicKeyDisabledAtWindowViolationError;
use crate::consensus::state::identity::identity_public_key_is_disabled_error::IdentityPublicKeyIsDisabledError;
use crate::consensus::state::identity::identity_public_key_is_read_only_error::IdentityPublicKeyIsReadOnlyError;
use crate::consensus::state::identity::invalid_identity_public_key_id_error::InvalidIdentityPublicKeyIdError;
use crate::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
use crate::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;
use crate::consensus::state::identity::{
    IdentityAlreadyExistsError, IdentityInsufficientBalanceError,
};
use crate::consensus::ConsensusError;

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

    #[error(transparent)]
    IdentityPublicKeyDisabledAtWindowViolationError(
        IdentityPublicKeyDisabledAtWindowViolationError,
    ),

    #[error(transparent)]
    IdentityPublicKeyIsReadOnlyError(IdentityPublicKeyIsReadOnlyError),

    #[error(transparent)]
    InvalidIdentityPublicKeyIdError(InvalidIdentityPublicKeyIdError),

    #[error(transparent)]
    MaxIdentityPublicKeyLimitReachedError(MaxIdentityPublicKeyLimitReachedError),

    #[error(transparent)]
    IdentityPublicKeyIsDisabledError(IdentityPublicKeyIsDisabledError),

    #[error(transparent)]
    DataTriggerError(DataTriggerError),
}

impl From<StateError> for ConsensusError {
    fn from(error: StateError) -> Self {
        Self::StateError(error)
    }
}

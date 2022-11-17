use thiserror::Error;

use crate::{identity::KeyID, prelude::Identifier};

use super::DataTriggerError;

#[derive(Error, Debug)]
pub enum StateError {
    // Document Errors
    #[error("Document {document_id} is already present")]
    DocumentAlreadyPresentError { document_id: Identifier },

    #[error("{document_id} document not found")]
    DocumentNotFoundError { document_id: Identifier },

    #[error("Provided document {document_id} owner ID {document_owner_id} mismatch with existing {existing_document_owner_id}")]
    DocumentOwnerMismatchError {
        document_id: Identifier,
        document_owner_id: Identifier,
        existing_document_owner_id: Identifier,
    },

    #[error("Document {document_id} createdAt and updatedAt timestamps are not equal")]
    DocumentTimestampMismatchError { document_id: Identifier },

    #[error("Document {document_id} {timestamp_name} timestamp {timestamp} are out of block time window from {time_window_start} and {time_window_end}")]
    DocumentTimestampWindowViolationError {
        timestamp_name: String,
        document_id: Identifier,
        timestamp: i64,
        time_window_start: i64,
        time_window_end: i64,
    },

    #[error("Document {document_id} has duplicate unique properties {duplicating_properties:?} with other documents")]
    DuplicateUniqueIndexError {
        document_id: Identifier,
        duplicating_properties: Vec<String>,
    },

    #[error(
        "Document {document_id} has invalid revision. The current revision is {current_revision}"
    )]
    InvalidDocumentRevisionError {
        document_id: Identifier,
        current_revision: u32,
    },

    #[error("Data Contract {data_contract_id} is already present")]
    DataContractAlreadyPresentError { data_contract_id: Identifier },

    #[error(transparent)]
    DataTriggerError(Box<DataTriggerError>),

    #[error(
        "Identity {identity_id} has invalid revision. The current revision is {current_revision}"
    )]
    InvalidIdentityRevisionError {
        identity_id: Identifier,
        current_revision: u32,
    },

    #[error("Duplicated public keys {duplicated_public_key_ids:?} found")]
    DuplicatedIdentityPublicKeyError {
        duplicated_public_key_ids: Vec<KeyID>,
    },

    #[error("Duplicated public keys ids {duplicated_ids:?} found")]
    DuplicatedIdentityPublicKeyIdError { duplicated_ids: Vec<KeyID> },

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
    MaxIdentityPublicKeyLimitReached { max_items: usize },

    #[error("Identity Public Key #{public_key_index} is disabled")]
    IdentityPublicKeyDisabledError { public_key_index: KeyID },
}

impl From<DataTriggerError> for StateError {
    fn from(v: DataTriggerError) -> Self {
        StateError::DataTriggerError(Box::new(v))
    }
}

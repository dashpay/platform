use thiserror::Error;

use crate::{document::document_transition::DocumentTransition, prelude::Identifier};

#[derive(Error, Debug)]
pub enum DataTriggerError {
    #[error("{message}")]
    DataTriggerConditionError {
        data_contract_id: Identifier,
        document_transition_id: Identifier,
        message: String,

        document_transition: Option<DocumentTransition>,
        owner_id: Option<Identifier>,
    },

    #[error("{message}")]
    DataTriggerExecutionError {
        data_contract_id: Identifier,
        document_transition_id: Identifier,
        message: String,
        // ? maybe we should replace with source
        execution_error: anyhow::Error,

        document_transition: Option<DocumentTransition>,
        owner_id: Option<Identifier>,
    },

    #[error("Data trigger have not returned any result")]
    DataTriggerInvalidResultError {
        data_contract_id: Identifier,
        document_transition_id: Identifier,

        document_transition: Option<DocumentTransition>,
        owner_id: Option<Identifier>,
    },
}

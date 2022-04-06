// Every DataTriggerError should contain:
// data_contract_id, document_transition_id, message

use std::error::Error;

use crate::prelude::Identifier;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataTriggerError {
    #[error("{message}")]
    DataTriggerConditionError {
        data_contract_id: Identifier,
        document_transition_id: Identifier,
        message: String,
    },

    #[error("{message}")]
    DataTriggerExecutionError {
        data_contract_id: Identifier,
        document_transition_id: Identifier,
        message: String,
        // ? maybe we should replace with source
        execution_error: Box<dyn Error>,
    },

    #[error("Data trigger have not returned any result")]
    DataTriggerInvalidResultError {
        data_contract_id: Identifier,
        document_transition_id: Identifier,
    },
}

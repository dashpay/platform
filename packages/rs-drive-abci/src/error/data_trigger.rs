use dpp::platform_value::Identifier;
use dpp::prelude::DocumentTransition;

/// Data trigger  errors
#[derive(Debug, thiserror::Error)]
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
        execution_error: String,

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

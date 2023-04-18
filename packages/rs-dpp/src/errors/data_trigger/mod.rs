use thiserror::Error;

use crate::{document::document_transition::DocumentTransition, prelude::Identifier};
use crate::document::document_transition::DocumentTransitionAction;
use platform_value::Error as ValueError;

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


/// Data trigger errors represent issues that occur while processing data triggers.
/// Data triggers are custom logic associated with the creation, modification, or deletion of documents.
#[derive(Debug, Error)]
pub enum DataTriggerActionError {
    /// An error occurred while evaluating the condition of the data trigger.
    #[error("{message}")]
    DataTriggerConditionError {
        /// The identifier of the associated data contract.
        data_contract_id: Identifier,
        /// The identifier of the associated document transition.
        document_transition_id: Identifier,
        /// A message describing the error.
        message: String,
        /// The document transition associated with the error, if available.
        document_transition: Option<DocumentTransitionAction>,
        /// The owner identifier associated with the error, if available.
        owner_id: Option<Identifier>,
    },

    /// An error occurred during the execution of the data trigger.
    #[error("{message}")]
    DataTriggerExecutionError {
        /// The identifier of the associated data contract.
        data_contract_id: Identifier,
        /// The identifier of the associated document transition.
        document_transition_id: Identifier,
        /// A message describing the error.
        message: String,
        /// A message describing the execution error.
        execution_error: String,
        /// The document transition associated with the error, if available.
        document_transition: Option<DocumentTransitionAction>,
        /// The owner identifier associated with the error, if available.
        owner_id: Option<Identifier>,
    },

    /// The data trigger did not return any result, which is invalid.
    #[error("Data trigger have not returned any result")]
    DataTriggerInvalidResultError {
        /// The identifier of the associated data contract.
        data_contract_id: Identifier,
        /// The identifier of the associated document transition.
        document_transition_id: Identifier,
        /// The document transition associated with the error, if available.
        document_transition: Option<DocumentTransitionAction>,
        /// The owner identifier associated with the error, if available.
        owner_id: Option<Identifier>,
    },

    /// A value error occurred while processing the data trigger.
    #[error("value error: {0}")]
    ValueError(#[from] ValueError),
}
use dpp::platform_value::Error as ValueError;
use dpp::platform_value::Identifier;
use dpp::prelude::DocumentTransition;

/// Data trigger errors represent issues that occur while processing data triggers.
/// Data triggers are custom logic associated with the creation, modification, or deletion of documents.
#[derive(Debug, thiserror::Error)]
pub enum DataTriggerError {
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
        document_transition: Option<DocumentTransition>,
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
        document_transition: Option<DocumentTransition>,
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
        document_transition: Option<DocumentTransition>,
        /// The owner identifier associated with the error, if available.
        owner_id: Option<Identifier>,
    },

    /// A value error occurred while processing the data trigger.
    #[error("value error: {0}")]
    ValueError(#[from] ValueError),
}

use crate::execution::validation::state_transition::documents_batch::data_triggers::{
    DataTrigger, DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use dpp::consensus::state::data_trigger::data_trigger_error::DataTriggerActionError;
use dpp::get_from_transition_action;
use dpp::identifier::Identifier;
use dpp::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::version::PlatformVersion;

/// A struct representing a data trigger on the blockchain.
///
/// The `DataTrigger` struct contains information about a data trigger, including the data contract ID, the document
/// type that the trigger handles, the kind of trigger, the action that triggered the trigger, and an optional
/// identifier for the top-level identity associated with the document.
#[derive(Default, Clone)]
pub struct DataTriggerBindingV0 {
    /// The identifier of the data contract associated with the trigger.
    pub data_contract_id: Identifier,
    /// The type of document that the trigger handles.
    pub document_type: String,
    /// The kind of data trigger.
    pub data_trigger: DataTrigger,
    /// The action that triggered the trigger.
    pub transition_action: Action,
}

pub trait DataTriggerBindingV0Getters {
    /// Executes the data trigger using the specified document transition and execution context.
    ///
    /// This function executes the data trigger using the specified `DocumentTransitionAction` and
    /// `DataTriggerExecutionContext`. It calls the `execute_trigger` function to perform the trigger
    /// execution, passing in the trigger kind, document transition, execution context, and top-level
    /// identity. It then returns a `DataTriggerExecutionResult` containing either a successful result or
    /// a `DataTriggerActionError`, indicating the failure of the trigger.
    ///
    /// # Arguments
    ///
    /// * `document_transition` - A reference to the document transition that triggered the data trigger.
    /// * `context` - A reference to the data trigger execution context.
    ///
    /// # Returns
    ///
    /// A `DataTriggerExecutionResult` containing either a successful result or a `DataTriggerActionError`,
    /// indicating the failure of the trigger.
    fn execute(
        &self,
        document_transition: &DocumentTransitionAction,
        context: &DataTriggerExecutionContext<'_>,
        platform_version: &PlatformVersion,
    ) -> DataTriggerExecutionResult;

    /// Checks whether the data trigger matches the specified data contract ID, document type, and action.
    ///
    /// This function compares the fields of the `DataTrigger` struct with the specified data contract ID, document type,
    /// and action to determine whether the trigger matches. It returns `true` if the trigger matches and `false` otherwise.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - A reference to the data contract ID to match.
    /// * `document_type` - A reference to the document type to match.
    /// * `transition_action` - The action to match.
    ///
    /// # Returns
    ///
    /// A boolean value indicating whether the trigger matches the specified data contract ID, document type, and action.
    fn is_matching(
        &self,
        data_contract_id: &Identifier,
        document_type: &str,
        transition_action: Action,
    ) -> bool;
}

impl DataTriggerBindingV0Getters for DataTriggerBindingV0 {
    fn execute(
        &self,
        document_transition: &DocumentTransitionAction,
        context: &DataTriggerExecutionContext<'_>,
        platform_version: &PlatformVersion,
    ) -> DataTriggerExecutionResult {
        let mut result = DataTriggerExecutionResult::default();

        match self.data_trigger(document_transition, context, platform_version) {
            Err(err) => {
                let consensus_error = DataTriggerActionError::DataTriggerExecutionError {
                    // TODO remove the clone
                    data_contract_id: context.data_contract.id.to_owned(),
                    document_transition_id: *get_from_transition_action!(document_transition, id),
                    message: err.to_string(),
                    execution_error: err.to_string(),
                    document_transition: None,
                    owner_id: None,
                };

                result.add_error(consensus_error);

                result
            }

            Ok(execution_result) => execution_result,
        }
    }

    fn is_matching(
        &self,
        data_contract_id: &Identifier,
        document_type: &str,
        transition_action: Action,
    ) -> bool {
        &self.data_contract_id == data_contract_id
            && self.document_type == document_type
            && self.transition_action == transition_action
    }
}

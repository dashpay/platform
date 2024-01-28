///! The `feature_flags_data_triggers` module contains data triggers related to feature flags.
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use dpp::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;

use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use drive::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::system_data_contracts::feature_flags_contract;
use dpp::system_data_contracts::feature_flags_contract::v1::document_types::update_consensus_params::properties
::PROPERTY_ENABLE_AT_HEIGHT;
use dpp::version::PlatformVersion;

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};

/// Creates a data trigger for handling feature flag documents.
///
/// The trigger is executed whenever a new feature flag document is created on the blockchain.
/// It performs various actions depending on the state of the document and the context in which it was created.
///
/// # Arguments
///
/// * `document_transition` - A reference to the document transition that triggered the data trigger.
/// * `context` - A reference to the data trigger execution context.
/// * `feature_flags_contract::OWNER_ID` - An optional identifier for the top-level identity associated with the feature flag
///   document (if one exists).
///
/// # Returns
///
/// A `DataTriggerExecutionResult` indicating the success or failure of the trigger execution.
pub fn create_feature_flag_data_trigger_v0(
    document_transition: &DocumentTransitionAction,
    context: &DataTriggerExecutionContext<'_>,
    _platform_version: &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error> {
    let mut result = DataTriggerExecutionResult::default();
    let data_contract_fetch_info = document_transition.base().data_contract_fetch_info();
    let data_contract = &data_contract_fetch_info.contract;

    let document_create_transition = match document_transition {
        DocumentTransitionAction::CreateAction(d) => d,
        _ => {
            return Err(Error::Execution(ExecutionError::DataTriggerExecutionError(
                format!(
                    "the Document Transition {} isn't 'CREATE",
                    document_transition.base().id()
                ),
            )))
        }
    };

    let data = document_create_transition.data();

    let enable_at_height: u64 = data.get_integer(PROPERTY_ENABLE_AT_HEIGHT).map_err(|_| {
        Error::Execution(ExecutionError::DataTriggerExecutionError(format!(
            "property missing for create_feature_flag_data_trigger '{}'",
            PROPERTY_ENABLE_AT_HEIGHT
        )))
    })?;

    let latest_block_height = context.platform.state.last_committed_height();

    if enable_at_height < latest_block_height {
        let err = DataTriggerConditionError::new(
            data_contract.id(),
            document_transition.base().id(),
            "This identity can't activate selected feature flag".to_string(),
        );

        result.add_error(err);

        return Ok(result);
    }

    if context.owner_id != &feature_flags_contract::OWNER_ID {
        let err = DataTriggerConditionError::new(
            data_contract.id(),
            document_transition.base().id(),
            "This identity can't activate selected feature flag".to_string(),
        );

        result.add_error(err);
    }

    Ok(result)
}

#[cfg(test)]
mod test {

    //todo: Ivan look at this!
    // #[test]
    // fn should_successfully_execute_on_dry_run() {
    //     let platform = TestPlatformBuilder::new()
    //         .build_with_mock_rpc()
    //         .set_initial_state_structure();
    //     let state_read_guard = platform.state.read().unwrap();
    //
    //     let platform_ref = PlatformStateRef {
    //         drive: &platform.drive,
    //         state: &state_read_guard,
    //         config: &platform.config,
    //     };
    //
    //     let platform_version = state_read_guard
    //         .current_platform_version()
    //         .expect("should return a platform version");
    //
    //     let mut transition_execution_context =
    //         StateTransitionExecutionContext::default_for_platform_version(platform_version)
    //             .unwrap();
    //     let data_contract = get_data_contract_fixture(
    //         None,
    //         state_read_guard.current_protocol_version_in_consensus(),
    //     )
    //     .data_contract_owned();
    //     let owner_id = data_contract.owner_id();
    //
    //     let base_transition: DocumentBaseTransitionAction = DocumentBaseTransitionActionV0 {
    //         id: Default::default(),
    //         document_type_name: "".to_string(),
    //         data_contract: Arc::new(DataContractFetchInfo::dpns_contract_fixture(
    //             platform_version.protocol_version,
    //         )),
    //     }
    //     .into();
    //
    //     let create_transition: DocumentCreateTransitionAction = DocumentCreateTransitionActionV0 {
    //         base: base_transition,
    //         created_at: None,
    //         updated_at: None,
    //         data: Default::default(),
    //     }
    //     .into();
    //
    //     transition_execution_context.enable_dry_run();
    //
    //     let document_transition = DocumentTransitionAction::CreateAction(create_transition);
    //     let data_trigger_context = DataTriggerExecutionContext {
    //         platform: &platform_ref,
    //         owner_id: &owner_id,
    //         state_transition_execution_context: &transition_execution_context,
    //         transaction: None,
    //     };
    //
    //     let result = create_feature_flag_data_trigger_v0(
    //         &document_transition,
    //         &data_trigger_context,
    //         platform_version,
    //     )
    //     .expect("the execution result should be returned");
    //
    //     assert!(result.is_valid());
    // }
}

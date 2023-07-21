///! The `feature_flags_data_triggers` module contains data triggers related to feature flags.
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::data_trigger::create_error;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

use dpp::get_from_transition_action;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use dpp::version::PlatformVersion;

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};

const PROPERTY_BLOCK_HEIGHT: &str = "height";
const PROPERTY_ENABLE_AT_HEIGHT: &str = "enableAtHeight";

/// Creates a data trigger for handling feature flag documents.
///
/// The trigger is executed whenever a new feature flag document is created on the blockchain.
/// It performs various actions depending on the state of the document and the context in which it was created.
///
/// # Arguments
///
/// * `document_transition` - A reference to the document transition that triggered the data trigger.
/// * `context` - A reference to the data trigger execution context.
/// * `top_level_identity` - An optional identifier for the top-level identity associated with the feature flag
///   document (if one exists).
///
/// # Returns
///
/// A `DataTriggerExecutionResult` indicating the success or failure of the trigger execution.
pub fn create_feature_flag_data_trigger(
    document_transition: &DocumentTransitionAction,
    context: &DataTriggerExecutionContext<'_>,
    _platform_version: &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error> {
    let mut result = DataTriggerExecutionResult::default();
    if context.state_transition_execution_context.is_dry_run() {
        return Ok(result);
    }

    let document_create_transition = match document_transition {
        DocumentTransitionAction::CreateAction(d) => d,
        _ => {
            return Err(Error::Execution(ExecutionError::DataTriggerExecutionError(
                format!(
                    "the Document Transition {} isn't 'CREATE",
                    get_from_transition_action!(document_transition, id)
                ),
            )))
        }
    };

    let data = &document_create_transition.data;

    let top_level_identity = top_level_identity.ok_or(Error::Execution(
        ExecutionError::DataTriggerExecutionError("top level identity isn't provided".to_string()),
    ))?;

    let enable_at_height: u64 = data.get_integer(PROPERTY_ENABLE_AT_HEIGHT).map_err(|_| {
        Error::Execution(ExecutionError::DataTriggerExecutionError(format!(
            "property missing for create_feature_flag_data_trigger '{}'",
            PROPERTY_ENABLE_AT_HEIGHT
        )))
    })?;

    let latest_block_height = context.platform.state.height();

    if enable_at_height < latest_block_height {
        let err = create_error(
            context,
            document_create_transition,
            "This identity can't activate selected feature flag".to_string(),
        );
        result.add_error(err);
        return Ok(result);
    }

    if context.owner_id != top_level_identity {
        let err = create_error(
            context,
            document_create_transition,
            "This Identity can't activate selected feature flag".to_string(),
        );
        result.add_error(err);
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use super::create_feature_flag_data_trigger;
    use crate::execution::validation::data_trigger::DataTriggerExecutionContext;
    use crate::platform_types::platform::PlatformStateRef;
    use crate::test::helpers::setup::TestPlatformBuilder;

    use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
    use dpp::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_successfully_execute_on_dry_run() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let state_read_guard = platform.state.read().unwrap();

        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_read_guard,
            config: &platform.config,
        };

        let transition_execution_context = StateTransitionExecutionContext::default();
        let data_contract = get_data_contract_fixture(None).data_contract;
        let owner_id = &data_contract.owner_id;

        let document_transition = DocumentTransitionAction::CreateAction(Default::default());
        let data_trigger_context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract,
            owner_id,
            state_transition_execution_context: &transition_execution_context,
            transaction: None,
        };

        transition_execution_context.enable_dry_run();

        let result = create_feature_flag_data_trigger(
            &document_transition,
            &data_trigger_context,
            PlatformVersion::first(),
        )
        .expect("the execution result should be returned");

        assert!(result.is_valid());
    }
}

use crate::error::data_trigger::DataTriggerError;
use crate::error::Error;
use dpp::document::document_transition::DocumentCreateTransitionAction;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::Identifier;
use dpp::prelude::DocumentTransition;

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};

const PROPERTY_BLOCK_HEIGHT: &str = "height";
const PROPERTY_ENABLE_AT_HEIGHT: &str = "enableAtHeight";

pub fn create_feature_flag_data_trigger<'a>(
    document_create_transition: &DocumentCreateTransitionAction,
    latest_block_height: u64,
    context: &DataTriggerExecutionContext<'a>,
    top_level_identity: &Identifier,
) -> Result<DataTriggerExecutionResult, Error> {
    let mut result = DataTriggerExecutionResult::default();
    if context.state_transition_execution_context.is_dry_run() {
        return Ok(result);
    }

    let enable_at_height: u64 = check_data_trigger_validation_result(
        document_create_transition
            .data
            .get_integer(PROPERTY_ENABLE_AT_HEIGHT),
    );

    if enable_at_height < latest_block_height {
        let err = create_error(
            context,
            dt_create,
            "This identity can't activate selected feature flag".to_string(),
        );
        result.add_error(err.into());
        return Ok(result);
    }

    if context.owner_id != top_level_identity {
        let err = create_error(
            context,
            dt_create,
            "This Identity can't activate selected feature flag".to_string(),
        );
        result.add_error(err.into());
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use super::create_feature_flag_data_trigger;
    use crate::execution::data_trigger::DataTriggerExecutionContext;
    use crate::{
        data_trigger::DataTriggerExecutionContext,
        document::document_transition::DocumentTransition,
        state_repository::MockStateRepositoryLike,
        state_transition::state_transition_execution_context::StateTransitionExecutionContext,
        tests::fixtures::get_data_contract_fixture,
    };
    use dpp::prelude::DocumentTransition;
    use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
    use dpp::tests::fixtures::get_data_contract_fixture;

    fn should_successfully_execute_on_dry_run() {
        let transition_execution_context = StateTransitionExecutionContext::default();
        let state_repository = MockStateRepositoryLike::new();
        let data_contract = get_data_contract_fixture(None);
        let owner_id = &data_contract.owner_id;

        let document_transition = DocumentTransition::Create(Default::default());
        let data_trigger_context = DataTriggerExecutionContext {
            data_contract: &data_contract,
            owner_id,
            drive: &state_repository,
            state_transition_execution_context: &transition_execution_context,
        };

        transition_execution_context.enable_dry_run();

        let result =
            create_feature_flag_data_trigger(&document_transition, &data_trigger_context, None)
                .expect("the execution result should be returned");

        assert!(result.is_ok());
    }
}

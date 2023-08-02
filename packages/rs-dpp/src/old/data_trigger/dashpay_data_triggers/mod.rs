use anyhow::{anyhow, bail};
use platform_value::btreemap_extensions::BTreeValueMapHelper;

use crate::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use crate::data_trigger::dashpay_data_triggers::property_names::CORE_HEIGHT_CREATED_AT;
use crate::{
    document::document_transition::DocumentTransition, get_from_transition, prelude::Identifier,
    state_repository::StateRepositoryLike, ProtocolError,
};

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};

const BLOCKS_SIZE_WINDOW: u32 = 8;
mod property_names {
    pub const TO_USER_ID: &str = "toUserId";
    pub const CORE_HEIGHT_CREATED_AT: &str = "coreHeightCreatedAt";
    pub const CORE_CHAIN_LOCKED_HEIGHT: &str = "coreChainLockedHeight";
}


pub fn get_data_trigger_error_from_execution_result(
    result: &DataTriggerExecutionResult,
    error_number: usize,
) -> &DataTriggerError {
    match result
        .errors
        .get(error_number)
        .expect("basic error should be found")
    {
        StateError::DataTriggerError(error) => error,
        _ => panic!(
            "error '{:?}' isn't a Data Trigger error",
            result.errors[error_number]
        ),
    }
}

pub async fn create_contact_request_data_trigger<'a, SR>(
    document_transition: &DocumentTransition,
    context: &DataTriggerExecutionContext<'a, SR>,
    _: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, anyhow::Error>
where
    SR: StateRepositoryLike,
{
    let mut result = DataTriggerExecutionResult::default();
    let is_dry_run = context.state_transition_execution_context.is_dry_run();
    let owner_id = context.owner_id;

    let document_create_transition = match document_transition {
        DocumentTransition::Create(d) => d,
        _ => bail!(
            "the Document Transition {} isn't 'CREATE",
            get_from_transition!(document_transition, id)
        ),
    };
    let data = document_create_transition.data.as_ref().ok_or_else(|| {
        anyhow!(
            "data isn't defined in Data Transition {}",
            document_create_transition.base.id
        )
    })?;

    let maybe_core_height_created_at: Option<u32> = data
        .get_optional_integer(CORE_HEIGHT_CREATED_AT)
        .map_err(ProtocolError::ValueError)?;
    let to_user_id = data.get_identifier(property_names::TO_USER_ID)?;

    if !is_dry_run {
        if owner_id == &to_user_id {
            result.add_error(
                DataTriggerConditionError::new(
                    context.data_contract.id,
                    document_create_transition.base.id,
                    format!("Identity {to_user_id} must not be equal to owner id"),
                )
                .into(),
            );

            return Ok(result);
        }

        if let Some(core_height_created_at) = maybe_core_height_created_at {
            let core_chain_locked_height = context
                .state_repository
                .fetch_latest_platform_core_chain_locked_height()
                .await?
                // is unwrap_or_default necessary?
                .unwrap_or_default();

            let height_window_start = core_chain_locked_height.saturating_sub(BLOCKS_SIZE_WINDOW);
            let height_window_end = core_chain_locked_height.saturating_add(BLOCKS_SIZE_WINDOW);

            if core_height_created_at < height_window_start
                || core_height_created_at > height_window_end
            {
                result.add_error(
                    DataTriggerConditionError::new(
                        context.data_contract.id,
                        document_create_transition.base.id,
                        format!(
                            "Core height {} is out of block height window from {} to {}",
                            core_height_created_at, height_window_start, height_window_end
                        ),
                    )
                    .into(),
                );

                return Ok(result);
            }
        }
    }

    //  toUserId identity exits
    let identity = context
        .state_repository
        .fetch_identity(
            &to_user_id,
            Some(context.state_transition_execution_context),
        )
        .await?;

    if !is_dry_run && identity.is_none() {
        result.add_error(
            DataTriggerConditionError::new(
                context.data_contract.id,
                document_create_transition.base.id,
                format!("Identity {to_user_id} doesn't exist"),
            )
            .into(),
        );

        return Ok(result);
    }

    Ok(result)
}

#[cfg(test)]
mod test {

    use platform_value::btreemap_extensions::BTreeValueMapHelper;
    use platform_value::platform_value;

    use super::create_contact_request_data_trigger;
    use crate::consensus::state::data_trigger::data_trigger_error::DataTriggerError;
    use crate::{
        data_trigger::DataTriggerExecutionContext,
        document::document_transition::Action,
        state_repository::MockStateRepositoryLike,
        state_transition::state_transition_execution_context::StateTransitionExecutionContext,
        tests::{
            fixtures::{
                get_contact_request_document_fixture, get_dashpay_contract_fixture,
                get_document_transitions_fixture, identity_fixture,
            },
            utils::get_data_trigger_error_from_execution_result,
        },
    };

    #[tokio::test]
    async fn should_successfully_execute_on_dry_run() {
        let mut contact_request_document = get_contact_request_document_fixture(None, None);
        contact_request_document
            .set(
                super::property_names::CORE_HEIGHT_CREATED_AT,
                platform_value!(10u32),
            )
            .expect("expected to set core height created at");
        let owner_id = &contact_request_document.owner_id();

        let document_transitions =
            get_document_transitions_fixture([(DocumentTransitionActionType::Create, vec![contact_request_document])]);
        let document_transition = document_transitions
            .get(0)
            .expect("document transition should be present");

        let data_contract = get_dashpay_contract_fixture(None).data_contract;
        let mut state_repository = MockStateRepositoryLike::new();
        state_repository
            .expect_fetch_identity()
            .returning(|_, _| Ok(None));
        let transition_execution_context = StateTransitionExecutionContext::default();

        let data_trigger_context = DataTriggerExecutionContext {
            data_contract: &data_contract,
            owner_id,
            state_repository: &state_repository,
            state_transition_execution_context: &transition_execution_context,
        };

        transition_execution_context.enable_dry_run();

        let result =
            create_contact_request_data_trigger(document_transition, &data_trigger_context, None)
                .await
                .expect("the execution result should be returned");

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn should_fail_if_owner_id_equals_to_user_id() {
        let mut contact_request_document = get_contact_request_document_fixture(None, None);
        let owner_id = contact_request_document.owner_id();
        contact_request_document
            .set("toUserId", platform_value::to_value(owner_id).unwrap())
            .expect("expected to set toUserId");

        let data_contract = get_dashpay_contract_fixture(None).data_contract;
        let document_transitions =
            get_document_transitions_fixture([(DocumentTransitionActionType::Create, vec![contact_request_document])]);
        let document_transition = document_transitions
            .get(0)
            .expect("document transition should be present");

        let transition_execution_context = StateTransitionExecutionContext::default();
        let identity_fixture = identity_fixture();
        let mut state_repository = MockStateRepositoryLike::new();

        state_repository
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity_fixture.clone())));
        state_repository
            .expect_fetch_latest_platform_core_chain_locked_height()
            .returning(|| Ok(Some(42)));

        let data_trigger_context = DataTriggerExecutionContext {
            data_contract: &data_contract,
            owner_id: &owner_id,
            state_repository: &state_repository,
            state_transition_execution_context: &transition_execution_context,
        };

        let dashpay_identity_id = data_trigger_context.owner_id.to_owned();

        let result = create_contact_request_data_trigger(
            document_transition,
            &data_trigger_context,
            Some(&dashpay_identity_id),
        )
        .await
        .expect("data trigger result should be returned");

        assert!(!result.is_ok());
        let data_trigger_error = get_data_trigger_error_from_execution_result(&result, 0);

        assert!(matches!(
            &data_trigger_error,
            &DataTriggerError::DataTriggerConditionError(e)  if {
                e.message() == &format!("Identity {owner_id} must not be equal to owner id")


            }
        ));
    }

    #[tokio::test]
    async fn should_fail_if_id_not_exists() {
        let contact_request_document = get_contact_request_document_fixture(None, None);
        let data_contract = get_dashpay_contract_fixture(None).data_contract;
        let owner_id = contact_request_document.owner_id();
        let contract_request_to_user_id = contact_request_document
            .document
            .properties
            .get_identifier("toUserId")
            .expect("expected to get toUserId");

        let document_transitions =
            get_document_transitions_fixture([(DocumentTransitionActionType::Create, vec![contact_request_document])]);
        let document_transition = document_transitions
            .get(0)
            .expect("document transition should be present");

        let transition_execution_context = StateTransitionExecutionContext::default();
        let mut state_repository = MockStateRepositoryLike::new();
        state_repository
            .expect_fetch_identity()
            .returning(|_, _| Ok(None));

        let data_trigger_context = DataTriggerExecutionContext {
            data_contract: &data_contract,
            owner_id: &owner_id,
            state_repository: &state_repository,
            state_transition_execution_context: &transition_execution_context,
        };

        let dashpay_identity_id = data_trigger_context.owner_id.to_owned();

        let result = create_contact_request_data_trigger(
            document_transition,
            &data_trigger_context,
            Some(&dashpay_identity_id),
        )
        .await
        .expect("data trigger result should be returned");

        assert!(!result.is_ok());
        let data_trigger_error = get_data_trigger_error_from_execution_result(&result, 0);

        assert!(matches!(
            &data_trigger_error,
            &DataTriggerError::DataTriggerConditionError(e)  if {
                e.message() == &format!("Identity {contract_request_to_user_id} doesn't exist")
            }
        ));
    }

    // TODO! implement remaining tests
}

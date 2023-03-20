use anyhow::{anyhow, bail};

use crate::{
    document::document_transition::DocumentTransition, errors::DataTriggerError,
    get_from_transition, prelude::Identifier, state_repository::StateRepositoryLike,
    util::json_value::JsonValueExt,
};

use super::{DataTriggerExecutionContext, DataTriggerExecutionResult};

const BLOCKS_SIZE_WINDOW: i64 = 8;
mod property_names {
    pub const TO_USER_ID: &str = "toUserId";
    pub const CORE_HEIGHT_CREATED_AT: &str = "coreHeightCreatedAt";
    pub const CORE_CHAIN_LOCKED_HEIGHT: &str = "coreChainLockedHeight";
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

    let maybe_core_height_created_at = data.get_i64(property_names::CORE_HEIGHT_CREATED_AT);
    let to_user_id_bytes = data.get_bytes(property_names::TO_USER_ID)?;
    let to_user_id = Identifier::from_bytes(&to_user_id_bytes)?;

    if !is_dry_run {
        if owner_id == &to_user_id {
            let err = DataTriggerError::DataTriggerConditionError {
                data_contract_id: context.data_contract.id,
                document_transition_id: document_create_transition.base.id,
                message: format!("Identity {to_user_id} must not be equal to owner id"),
                document_transition: Some(DocumentTransition::Create(
                    document_create_transition.clone(),
                )),
                owner_id: Some(*context.owner_id),
            };
            result.add_error(err.into());
            return Ok(result);
        }

        if let Ok(core_height_created_at) = maybe_core_height_created_at {
            let core_chain_locked_height = context
                .state_repository
                .fetch_latest_platform_core_chain_locked_height()
                .await?
                // is unwrap_or_default necessary?
                .unwrap_or_default() as i64;

            let height_window_start = core_chain_locked_height - BLOCKS_SIZE_WINDOW;
            let height_window_end = core_chain_locked_height + BLOCKS_SIZE_WINDOW;

            if core_height_created_at < height_window_start
                || core_height_created_at > height_window_end
            {
                let err = DataTriggerError::DataTriggerConditionError {
                    data_contract_id: context.data_contract.id,
                    document_transition_id: document_create_transition.base.id,
                    message: format!(
                        "Core height {} is out of block height window from {} to {}",
                        core_height_created_at, height_window_start, height_window_end
                    ),
                    document_transition: Some(DocumentTransition::Create(
                        document_create_transition.clone(),
                    )),
                    owner_id: Some(*context.owner_id),
                };
                result.add_error(err.into());
                return Ok(result);
            }
        }
    }

    //  toUserId identity exits
    let identity = context
        .state_repository
        .fetch_identity(&to_user_id, context.state_transition_execution_context)
        .await?;

    if !is_dry_run && identity.is_none() {
        let err = DataTriggerError::DataTriggerConditionError {
            data_contract_id: context.data_contract.id,
            document_transition_id: document_create_transition.base.id,
            message: format!("Identity {to_user_id} doesn't exist"),
            document_transition: Some(DocumentTransition::Create(
                document_create_transition.clone(),
            )),
            owner_id: Some(*context.owner_id),
        };
        result.add_error(err.into());
        return Ok(result);
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use super::create_contact_request_data_trigger;
    use crate::{
        data_trigger::DataTriggerExecutionContext,
        document::document_transition::Action,
        prelude::Identifier,
        state_repository::MockStateRepositoryLike,
        state_transition::state_transition_execution_context::StateTransitionExecutionContext,
        tests::{
            fixtures::{
                get_contact_request_document_fixture, get_dashpay_contract_fixture,
                get_document_transitions_fixture, identity_fixture,
            },
            utils::get_data_trigger_error_from_execution_result,
        },
        DataTriggerError,
    };

    #[tokio::test]
    async fn should_successfully_execute_on_dry_run() {
        let mut contact_request_document = get_contact_request_document_fixture(None, None);
        contact_request_document
            .set(super::property_names::CORE_HEIGHT_CREATED_AT, json!(10))
            .expect("the property should be set");
        let owner_id = contact_request_document.owner_id.to_owned();

        let document_transitions =
            get_document_transitions_fixture([(Action::Create, vec![contact_request_document])]);
        let document_transition = document_transitions
            .get(0)
            .expect("document transition should be present");

        let data_contract = get_dashpay_contract_fixture(None);
        let mut state_repository = MockStateRepositoryLike::new();
        state_repository
            .expect_fetch_identity()
            .returning(|_, _| Ok(None));
        let transition_execution_context = StateTransitionExecutionContext::default();

        let data_trigger_context = DataTriggerExecutionContext {
            data_contract: &data_contract,
            owner_id: &owner_id,
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
        let owner_id = contact_request_document.owner_id.to_owned();
        contact_request_document
            .set(
                "toUserId",
                serde_json::to_value(owner_id.as_bytes()).unwrap(),
            )
            .expect("property 'toUserId' must be set");

        let data_contract = get_dashpay_contract_fixture(None);
        let document_transitions =
            get_document_transitions_fixture([(Action::Create, vec![contact_request_document])]);
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
            &DataTriggerError::DataTriggerConditionError { message, .. }  if {
                message == &format!("Identity {owner_id} must not be equal to owner id")


            }
        ));
    }

    #[tokio::test]
    async fn should_fail_if_id_not_exists() {
        let contact_request_document = get_contact_request_document_fixture(None, None);
        let data_contract = get_dashpay_contract_fixture(None);
        let owner_id = contact_request_document.owner_id.to_owned();
        let contract_request_to_user_id =
            to_identifier(contact_request_document.get("toUserId").unwrap().to_owned());

        let document_transitions =
            get_document_transitions_fixture([(Action::Create, vec![contact_request_document])]);
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
            &DataTriggerError::DataTriggerConditionError { message, .. }  if {
                message == &format!("Identity {contract_request_to_user_id} doesn't exist")


            }
        ));
    }

    fn to_identifier(value: Value) -> Identifier {
        let bytes: Vec<u8> = serde_json::from_value(value).expect("value must be bytes");
        Identifier::from_bytes(&bytes).expect("should be valid identifier")
    }

    // TODO! implement remaining tests
}

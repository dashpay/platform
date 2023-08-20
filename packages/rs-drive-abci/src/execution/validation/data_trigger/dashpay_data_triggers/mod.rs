use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::data_trigger::dashpay_data_triggers::property_names::CORE_HEIGHT_CREATED_AT;
use crate::execution::validation::data_trigger::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use dpp::consensus::state::data_trigger::data_trigger_error::DataTriggerActionError;
use dpp::document::document_transition::DocumentTransitionAction;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::Identifier;
use dpp::{get_from_transition_action, ProtocolError};

const BLOCKS_SIZE_WINDOW: u32 = 8;
mod property_names {
    pub const TO_USER_ID: &str = "toUserId";
    pub const CORE_HEIGHT_CREATED_AT: &str = "coreHeightCreatedAt";
    pub const CORE_CHAIN_LOCKED_HEIGHT: &str = "coreChainLockedHeight";
}

/// Creates a data trigger for handling contact request documents.
///
/// The trigger is executed whenever a new contact request document is created on the blockchain.
/// It sends a notification to the user specified in the document, notifying them that someone
/// has requested to add them as a contact.
///
/// # Arguments
///
/// * `document_transition` - A reference to the document transition that triggered the data trigger.
/// * `context` - A reference to the data trigger execution context.
/// * `_` - An unused parameter for the owner ID (which is not needed for this trigger).
///
/// # Returns
///
/// A `DataTriggerExecutionResult` indicating the success or failure of the trigger execution.
pub fn create_contact_request_data_trigger(
    document_transition: &DocumentTransitionAction,
    context: &DataTriggerExecutionContext<'_>,
    _: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, Error> {
    let mut result = DataTriggerExecutionResult::default();
    let is_dry_run = context.state_transition_execution_context.is_dry_run();
    let owner_id = context.owner_id;

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

    let maybe_core_height_created_at: Option<u32> = data
        .get_optional_integer(CORE_HEIGHT_CREATED_AT)
        .map_err(ProtocolError::ValueError)?;
    let to_user_id = data
        .get_identifier(property_names::TO_USER_ID)
        .map_err(ProtocolError::ValueError)?;

    if !is_dry_run {
        if owner_id == &to_user_id {
            let err = DataTriggerActionError::DataTriggerConditionError {
                data_contract_id: context.data_contract.id,
                document_transition_id: document_create_transition.base.id,
                message: format!("Identity {to_user_id} must not be equal to owner id"),
                document_transition: Some(DocumentTransitionAction::CreateAction(
                    document_create_transition.clone(),
                )),
                owner_id: Some(*context.owner_id),
            };
            result.add_error(err);
            return Ok(result);
        }

        if let Some(core_height_created_at) = maybe_core_height_created_at {
            let core_chain_locked_height = context.platform.state.core_height();

            let height_window_start = core_chain_locked_height.saturating_sub(BLOCKS_SIZE_WINDOW);
            let height_window_end = core_chain_locked_height.saturating_add(BLOCKS_SIZE_WINDOW);

            if core_height_created_at < height_window_start
                || core_height_created_at > height_window_end
            {
                let err = DataTriggerActionError::DataTriggerConditionError {
                    data_contract_id: context.data_contract.id,
                    document_transition_id: document_create_transition.base.id,
                    message: format!(
                        "Core height {} is out of block height window from {} to {}",
                        core_height_created_at, height_window_start, height_window_end
                    ),
                    document_transition: Some(DocumentTransitionAction::CreateAction(
                        document_create_transition.clone(),
                    )),
                    owner_id: Some(*context.owner_id),
                };
                result.add_error(err);
                return Ok(result);
            }
        }
    }

    //  toUserId identity exits
    let identity = context
        .platform
        .drive
        .fetch_identity_balance(to_user_id.to_buffer(), context.transaction)?;

    if !is_dry_run && identity.is_none() {
        let err = DataTriggerActionError::DataTriggerConditionError {
            data_contract_id: context.data_contract.id,
            document_transition_id: document_create_transition.base.id,
            message: format!("Identity {to_user_id} doesn't exist"),
            document_transition: Some(DocumentTransitionAction::CreateAction(
                document_create_transition.clone(),
            )),
            owner_id: Some(*context.owner_id),
        };
        result.add_error(err);
        return Ok(result);
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use crate::execution::validation::data_trigger::dashpay_data_triggers::create_contact_request_data_trigger;
    use crate::execution::validation::data_trigger::DataTriggerExecutionContext;
    use crate::platform_types::platform::PlatformStateRef;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::{BlockInfo, ExtendedBlockInfo};
    use dpp::document::document_transition::{Action, DocumentCreateTransitionAction};
    use dpp::errors::consensus::state::data_trigger::data_trigger_error::DataTriggerActionError;
    use dpp::platform_value;
    use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
    use dpp::platform_value::platform_value;
    use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
    use dpp::tests::fixtures::{
        get_contact_request_document_fixture, get_dashpay_contract_fixture,
        get_document_transitions_fixture, identity_fixture,
    };

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

        let mut contact_request_document = get_contact_request_document_fixture(None, None);
        contact_request_document
            .set(
                super::property_names::CORE_HEIGHT_CREATED_AT,
                platform_value!(10u32),
            )
            .expect("expected to set core height created at");
        let owner_id = &contact_request_document.owner_id();

        let document_transitions =
            get_document_transitions_fixture([(Action::Create, vec![contact_request_document])]);
        let document_transition = document_transitions
            .get(0)
            .expect("document transition should be present");

        let document_create_transition = document_transition
            .as_transition_create()
            .expect("expected a document create transition");

        let data_contract = get_dashpay_contract_fixture(None);

        let transition_execution_context = StateTransitionExecutionContext::default();

        let data_trigger_context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract.data_contract,
            owner_id,
            state_transition_execution_context: &transition_execution_context,
            transaction: None,
        };

        transition_execution_context.enable_dry_run();

        let result = create_contact_request_data_trigger(
            &DocumentCreateTransitionAction::from(document_create_transition).into(),
            &data_trigger_context,
            None,
        )
        .expect("the execution result should be returned");

        assert!(result.is_valid());
    }

    #[test]
    fn should_fail_if_owner_id_equals_to_user_id() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let mut state_write_guard = platform.state.write().unwrap();

        state_write_guard.last_committed_block_info = Some(ExtendedBlockInfo {
            basic_info: BlockInfo {
                time_ms: 500000,
                height: 100,
                core_height: 42,
                epoch: Default::default(),
            },
            app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
            quorum_hash: [0u8; 32],
            block_id_hash: [0u8; 32],
            signature: [0u8; 96],
            round: 0,
        });
        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_write_guard,
            config: &platform.config,
        };

        let mut contact_request_document = get_contact_request_document_fixture(None, None);
        let owner_id = contact_request_document.owner_id();
        contact_request_document
            .set("toUserId", platform_value::to_value(owner_id).unwrap())
            .expect("expected to set toUserId");

        let data_contract = get_dashpay_contract_fixture(None);
        let document_transitions =
            get_document_transitions_fixture([(Action::Create, vec![contact_request_document])]);
        let document_transition = document_transitions
            .get(0)
            .expect("document transition should be present");

        let document_create_transition = document_transition
            .as_transition_create()
            .expect("expected a document create transition");

        let transition_execution_context = StateTransitionExecutionContext::default();
        let identity_fixture = identity_fixture();

        platform
            .drive
            .add_new_identity(identity_fixture, &BlockInfo::default(), true, None)
            .expect("expected to insert identity");

        let data_trigger_context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract.data_contract,
            owner_id: &owner_id,
            state_transition_execution_context: &transition_execution_context,
            transaction: None,
        };

        let dashpay_identity_id = data_trigger_context.owner_id.to_owned();

        let result = create_contact_request_data_trigger(
            &DocumentCreateTransitionAction::from(document_create_transition).into(),
            &data_trigger_context,
            Some(&dashpay_identity_id),
        )
        .expect("data trigger result should be returned");

        assert!(!result.is_valid());

        assert!(matches!(
            &result.errors.first().unwrap(),
            &DataTriggerActionError::DataTriggerConditionError { message, .. }  if {
                message == &format!("Identity {owner_id} must not be equal to owner id")


            }
        ));
    }

    #[test]
    fn should_fail_if_id_not_exists() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let mut state_write_guard = platform.state.write().unwrap();

        state_write_guard.last_committed_block_info = Some(ExtendedBlockInfo {
            basic_info: BlockInfo {
                time_ms: 500000,
                height: 100,
                core_height: 42,
                epoch: Default::default(),
            },
            app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
            quorum_hash: [0u8; 32],
            block_id_hash: [0u8; 32],
            signature: [0u8; 96],
            round: 0,
        });

        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_write_guard,
            config: &platform.config,
        };

        let contact_request_document = get_contact_request_document_fixture(None, None);
        let data_contract = get_dashpay_contract_fixture(None);
        let owner_id = contact_request_document.owner_id();
        let contract_request_to_user_id = contact_request_document
            .document
            .properties
            .get_identifier("toUserId")
            .expect("expected to get toUserId");

        let document_transitions =
            get_document_transitions_fixture([(Action::Create, vec![contact_request_document])]);
        let document_transition = document_transitions
            .get(0)
            .expect("document transition should be present");

        let document_create_transition = document_transition
            .as_transition_create()
            .expect("expected a document create transition");

        let transition_execution_context = StateTransitionExecutionContext::default();

        let data_trigger_context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract.data_contract,
            owner_id: &owner_id,
            state_transition_execution_context: &transition_execution_context,
            transaction: None,
        };

        let dashpay_identity_id = data_trigger_context.owner_id.to_owned();

        let result = create_contact_request_data_trigger(
            &DocumentCreateTransitionAction::from(document_create_transition).into(),
            &data_trigger_context,
            Some(&dashpay_identity_id),
        )
        .expect("data trigger result should be returned");

        assert!(!result.is_valid());
        let data_trigger_error = &result.errors[0];

        assert!(matches!(
            data_trigger_error,
            DataTriggerActionError::DataTriggerConditionError { message, .. }  if {
                message == &format!("Identity {contract_request_to_user_id} doesn't exist")


            }
        ));
    }

    // TODO! implement remaining tests
}

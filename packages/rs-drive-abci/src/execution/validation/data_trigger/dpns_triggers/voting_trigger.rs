use crate::error::Error;
use crate::execution::validation::data_trigger::{
    DataTriggerExecutionContext, DataTriggerExecutionResult,
};
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::epoch::Epoch;
use dpp::consensus::state::data_trigger::data_trigger_error::DataTriggerActionError;
use dpp::document::document_transition::DocumentTransitionAction;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::Identifier;

const BLOCKS_SIZE_WINDOW: u32 = 8;
mod property_names {
    pub const TO_USER_ID: &str = "toUserId";
    pub const CORE_HEIGHT_CREATED_AT: &str = "coreHeightCreatedAt";
    pub const CORE_CHAIN_LOCKED_HEIGHT: &str = "coreChainLockedHeight";
}

// TODO: move it to another file
impl PlatformStateRef<'_> {
    /// Returns the current height of the blockchain.
    pub fn epoch(&self) -> Epoch {
        self.state.epoch()
    }
}

pub fn run_name_register_trigger(
    document_transition: &DocumentTransitionAction,
    context: &DataTriggerExecutionContext<'_>,
    _: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, Error> {
    let mut data_trigger_execution_result = DataTriggerExecutionResult::default();

    let current_epoch = context.current_epoch();

    if current_epoch.index != 0 {
        // As per spec, the voting for names is only allowed in the first epoch.
        return Ok(data_trigger_execution_result);
    }

    let DocumentTransitionAction::CreateAction(document_create_action) = document_transition else {
        // Not a name registration actions, as it doesn't create a document.
        return Ok(data_trigger_execution_result);
    };

    // TODO: I guess preorders can be created, it's the actual domain document that needs to be
    //  voted on
    if document_create_action.base.document_type_name != "domain" {
        // Not a name registration document.
        return Ok(data_trigger_execution_result);
    };

    println!("document_create_action: {:?}", document_create_action);

    // Check votes here
    data_trigger_execution_result.add_error(DataTriggerActionError::DataTriggerExecutionError {
        data_contract_id: context.data_contract.id.clone(),
        document_transition_id: document_create_action.base.id.clone(),
        message: "Vote didn't happen".to_string(),
        execution_error: "Vote didn't happen".to_string(),
        document_transition: Some(document_transition.clone()),
        owner_id: None,
    });

    Ok(data_trigger_execution_result)
}

#[cfg(test)]
mod test {
    use crate::execution::validation::data_trigger::dashpay_data_triggers::create_contact_request_data_trigger;
    use crate::execution::validation::data_trigger::dpns_triggers::run_name_register_trigger;
    use crate::execution::validation::data_trigger::DataTriggerExecutionContext;
    use crate::platform_types::platform::PlatformStateRef;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::{BlockInfo, ExtendedBlockInfo};
    use dpp::block::epoch::Epoch;
    use dpp::document::document_transition::{Action, DocumentCreateTransitionAction};
    use dpp::errors::consensus::state::data_trigger::data_trigger_error::DataTriggerActionError;
    use dpp::platform_value;
    use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
    use dpp::platform_value::platform_value;
    use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
    use dpp::tests::fixtures::{
        get_contact_request_document_fixture, get_dashpay_contract_fixture,
        get_document_transitions_fixture, get_dpns_data_contract_fixture,
        get_dpns_parent_document_fixture, get_dpns_preorder_document_fixture, identity_fixture,
        ParentDocumentOptions,
    };

    #[test]
    fn should_return_error_if_can_not_get_epoch_info() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let state_read_guard = platform.state.read().unwrap();
        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_read_guard,
            config: &platform.config,
        };
    }

    #[test]
    fn should_prevent_users_from_registering_names_on_epoch_0() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let state_read_guard = platform.state.read().unwrap();
        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_read_guard,
            config: &platform.config,
        };

        let mut domain_document =
            get_dpns_parent_document_fixture(ParentDocumentOptions::default());
        domain_document
            .set(
                super::property_names::CORE_HEIGHT_CREATED_AT,
                platform_value!(10u32),
            )
            .expect("expected to set core height created at");
        let owner_id = &domain_document.owner_id();

        let document_transitions =
            get_document_transitions_fixture([(Action::Create, vec![domain_document])]);
        let document_transition = document_transitions
            .get(0)
            .expect("document transition should be present");

        let document_create_transition = document_transition
            .as_transition_create()
            .expect("expected a document create transition");

        let data_contract = get_dpns_data_contract_fixture(None);

        let transition_execution_context = StateTransitionExecutionContext::default();

        let data_trigger_context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract.data_contract,
            owner_id,
            state_transition_execution_context: &transition_execution_context,
            transaction: None,
        };

        transition_execution_context.enable_dry_run();

        let result = run_name_register_trigger(
            &DocumentCreateTransitionAction::from(document_create_transition).into(),
            &data_trigger_context,
            None,
        )
        .expect("the execution result should be returned");

        assert!(!result.is_valid());

        let data_trigger_error = &result.errors[0];
        match data_trigger_error {
            DataTriggerActionError::DataTriggerExecutionError { message, .. } => {
                assert_eq!(message, "Vote didn't happen");
            }
            _ => {
                panic!("Expected DataTriggerExecutionError");
            }
        }
    }

    #[test]
    fn should_allow_registering_names_after_epoch_0() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let mut state_guard = platform.state.write().unwrap();

        state_guard.last_committed_block_info = Some(ExtendedBlockInfo {
            basic_info: BlockInfo {
                time_ms: 500000,
                height: 100,
                core_height: 42,
                epoch: Epoch::new(1).expect("expected to create epoch"),
            },
            app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
            quorum_hash: [0u8; 32],
            block_id_hash: [0u8; 32],
            signature: [0u8; 96],
            round: 0,
        });

        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_guard,
            config: &platform.config,
        };

        let mut domain_document =
            get_dpns_parent_document_fixture(ParentDocumentOptions::default());
        domain_document
            .set(
                super::property_names::CORE_HEIGHT_CREATED_AT,
                platform_value!(10u32),
            )
            .expect("expected to set core height created at");
        let owner_id = &domain_document.owner_id();

        let document_transitions =
            get_document_transitions_fixture([(Action::Create, vec![domain_document])]);
        let document_transition = document_transitions
            .get(0)
            .expect("document transition should be present");

        let document_create_transition = document_transition
            .as_transition_create()
            .expect("expected a document create transition");

        let data_contract = get_dpns_data_contract_fixture(None);

        let transition_execution_context = StateTransitionExecutionContext::default();

        let data_trigger_context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract.data_contract,
            owner_id,
            state_transition_execution_context: &transition_execution_context,
            transaction: None,
        };

        transition_execution_context.enable_dry_run();

        let result = run_name_register_trigger(
            &DocumentCreateTransitionAction::from(document_create_transition).into(),
            &data_trigger_context,
            None,
        )
        .expect("the execution result should be returned");

        assert!(result.is_valid());
    }

    #[test]
    fn should_always_return_valid_results_for_preorder_at_epoch_0() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let state_read_guard = platform.state.read().unwrap();
        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state_read_guard,
            config: &platform.config,
        };

        let (mut preorder_document, preorder_salt) =
            get_dpns_preorder_document_fixture(ParentDocumentOptions::default());
        preorder_document
            .set(
                super::property_names::CORE_HEIGHT_CREATED_AT,
                platform_value!(10u32),
            )
            .expect("expected to set core height created at");
        let owner_id = &preorder_document.owner_id();

        let document_transitions =
            get_document_transitions_fixture([(Action::Create, vec![preorder_document])]);
        let document_transition = document_transitions
            .get(0)
            .expect("document transition should be present");

        let document_create_transition = document_transition
            .as_transition_create()
            .expect("expected a document create transition");

        let data_contract = get_dpns_data_contract_fixture(None);

        let transition_execution_context = StateTransitionExecutionContext::default();

        let data_trigger_context = DataTriggerExecutionContext {
            platform: &platform_ref,
            data_contract: &data_contract.data_contract,
            owner_id,
            state_transition_execution_context: &transition_execution_context,
            transaction: None,
        };

        transition_execution_context.enable_dry_run();

        let result = run_name_register_trigger(
            &DocumentCreateTransitionAction::from(document_create_transition).into(),
            &data_trigger_context,
            None,
        )
        .expect("the execution result should be returned");

        assert!(result.is_valid());
    }

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
        // platform.state.last_committed_block_info.unwrap().basic_info.epoch = 1;
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
    // TODO! implement remaining tests
}

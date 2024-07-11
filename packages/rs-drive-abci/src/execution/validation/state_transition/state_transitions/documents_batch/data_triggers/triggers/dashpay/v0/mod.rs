//! The `dashpay_data_triggers` module contains data triggers specific to the DashPay data contract.
use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;

use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::ProtocolError;
use drive::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use drive::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use dpp::system_data_contracts::dashpay_contract::v1::document_types::contact_request::properties
::{TO_USER_ID};
use dpp::version::PlatformVersion;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
use crate::execution::validation::state_transition::documents_batch::data_triggers::{DataTriggerExecutionContext, DataTriggerExecutionResult};

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
#[inline(always)]
pub(super) fn create_contact_request_data_trigger_v0(
    document_transition: &DocumentTransitionAction,
    context: &DataTriggerExecutionContext<'_>,
    platform_version: &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error> {
    let data_contract_fetch_info = document_transition
        .base()
        .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "expecting action to have a base",
        )))?
        .data_contract_fetch_info();
    let data_contract = &data_contract_fetch_info.contract;
    let mut result = DataTriggerExecutionResult::default();
    let is_dry_run = context.state_transition_execution_context.in_dry_run();
    let owner_id = context.owner_id;

    let DocumentTransitionAction::CreateAction(document_create_transition) = document_transition
    else {
        return Err(Error::Execution(ExecutionError::DataTriggerExecutionError(
            format!(
                "the Document Transition {} isn't 'CREATE",
                document_transition
                    .base()
                    .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "expecting action to have a base"
                    )))?
                    .id()
            ),
        )));
    };

    let data = document_create_transition.data();

    let to_user_id = data
        .get_identifier(TO_USER_ID)
        .map_err(ProtocolError::ValueError)?;

    // You shouldn't create a contract request to yourself
    if !is_dry_run && owner_id == &to_user_id {
        let err = DataTriggerConditionError::new(
            data_contract.id(),
            document_transition
                .base()
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "expecting action to have a base",
                )))?
                .id(),
            format!("Identity {to_user_id} must not be equal to owner id"),
        );

        result.add_error(err);

        return Ok(result);
    }

    // TODO: Calculate fee operations

    // Recipient identity must exist
    let to_identity = context.platform.drive.fetch_identity_balance(
        to_user_id.to_buffer(),
        context.transaction,
        platform_version,
    )?;

    if !is_dry_run && to_identity.is_none() {
        let err = DataTriggerConditionError::new(
            data_contract.id(),
            document_create_transition.base().id(),
            format!("Identity {to_user_id} doesn't exist"),
        );

        result.add_error(err);

        return Ok(result);
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use dpp::block::block_info::BlockInfo;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use dpp::document::{DocumentV0Getters, DocumentV0Setters};
    use dpp::platform_value;
    use dpp::platform_value::{Bytes32};
    use drive::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentCreateTransitionAction;
    use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionActionType;
    use crate::execution::validation::state_transition::documents_batch::data_triggers::triggers::dashpay::create_contact_request_data_trigger;
    use crate::platform_types::platform::PlatformStateRef;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use super::*;
    use dpp::errors::consensus::state::data_trigger::DataTriggerError;
    use dpp::tests::fixtures::{get_contact_request_document_fixture, get_dashpay_contract_fixture, get_document_transitions_fixture, get_identity_fixture};
    use dpp::version::DefaultForPlatformVersion;
    use drive::drive::contract::DataContractFetchInfo;
    use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
    use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

    #[test]
    fn should_successfully_execute_on_dry_run() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let mut nonce_counter = BTreeMap::new();
        let state = platform.state.load();
        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state,
            config: &platform.config,
        };
        let protocol_version = state.current_protocol_version_in_consensus();
        let platform_version = state
            .current_platform_version()
            .expect("should return a platform version");

        let contact_request_document = get_contact_request_document_fixture(
            None,
            0,
            None,
            state.current_protocol_version_in_consensus(),
        );
        let owner_id = &contact_request_document.owner_id();

        let data_contract =
            get_dashpay_contract_fixture(None, 0, protocol_version).data_contract_owned();
        let document_type = data_contract
            .document_type_for_name("contactRequest")
            .expect("expected a contact request");

        let document_transitions = get_document_transitions_fixture(
            [(
                DocumentTransitionActionType::Create,
                vec![(contact_request_document, document_type, Bytes32::default())],
            )],
            &mut nonce_counter,
        );
        let document_transition = document_transitions
            .first()
            .expect("document transition should be present");

        let document_create_transition = document_transition
            .as_transition_create()
            .expect("expected a document create transition");

        let mut transition_execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .unwrap();

        transition_execution_context.enable_dry_run();

        let data_trigger_context = DataTriggerExecutionContext {
            platform: &platform_ref,
            owner_id,
            state_transition_execution_context: &transition_execution_context,
            transaction: None,
        };

        let result = create_contact_request_data_trigger(
            &DocumentCreateTransitionAction::from_document_borrowed_create_transition_with_contract_lookup(&platform.drive, None, document_create_transition, &BlockInfo::default(), |_identifier| {
                Ok(Arc::new(DataContractFetchInfo::dashpay_contract_fixture(protocol_version)))
            }, platform_version).expect("expected to create action").0.into(),
            &data_trigger_context,
            platform_version,
        )
        .expect("the execution result should be returned");

        assert!(result.is_valid());
    }

    #[test]
    fn should_return_invalid_result_if_owner_id_equals_to_user_id() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let mut nonce_counter = BTreeMap::new();

        let platform_state = platform.state.load();
        let mut platform_state = (**platform_state).clone();

        platform_state.set_last_committed_block_info(Some(
            ExtendedBlockInfoV0 {
                basic_info: BlockInfo {
                    time_ms: 500000,
                    height: 100,
                    core_height: 42,
                    epoch: Default::default(),
                },
                app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
                quorum_hash: [0u8; 32],
                block_id_hash: [0u8; 32],
                proposer_pro_tx_hash: [0u8; 32],
                signature: [0u8; 96],
                round: 0,
            }
            .into(),
        ));
        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &platform_state,
            config: &platform.config,
        };
        let protocol_version = platform_state.current_protocol_version_in_consensus();
        let platform_version = platform_state
            .current_platform_version()
            .expect("should return a platform version");

        let mut contact_request_document = get_contact_request_document_fixture(
            None,
            0,
            None,
            platform_state.current_protocol_version_in_consensus(),
        );
        let owner_id = contact_request_document.owner_id();
        contact_request_document.set("toUserId", platform_value::to_value(owner_id).unwrap());

        let data_contract = get_dashpay_contract_fixture(
            None,
            0,
            platform_state.current_protocol_version_in_consensus(),
        )
        .data_contract_owned();
        let document_type = data_contract
            .document_type_for_name("contactRequest")
            .expect("expected a contact request");

        let document_transitions = get_document_transitions_fixture(
            [(
                DocumentTransitionActionType::Create,
                vec![(contact_request_document, document_type, Bytes32::default())],
            )],
            &mut nonce_counter,
        );
        let document_transition = document_transitions
            .first()
            .expect("document transition should be present");

        let document_create_transition = document_transition
            .as_transition_create()
            .expect("expected a document create transition");

        let transition_execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .unwrap();
        let identity_fixture =
            get_identity_fixture(platform_state.current_protocol_version_in_consensus())
                .expect("expected to get identity fixture");

        platform
            .drive
            .add_new_identity(
                identity_fixture,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_state.current_platform_version().unwrap(),
            )
            .expect("expected to insert identity");

        let data_trigger_context = DataTriggerExecutionContext {
            platform: &platform_ref,
            owner_id: &owner_id,
            state_transition_execution_context: &transition_execution_context,
            transaction: None,
        };

        let _dashpay_identity_id = data_trigger_context.owner_id.to_owned();

        let result = create_contact_request_data_trigger(
            &DocumentCreateTransitionAction::from_document_borrowed_create_transition_with_contract_lookup(&platform.drive, None, document_create_transition, &BlockInfo::default(), |_identifier| {
                Ok(Arc::new(DataContractFetchInfo::dashpay_contract_fixture(protocol_version)))
            }, platform_version).expect("expected to create action").0.into(),
            &data_trigger_context,
            platform_version,
        )
        .expect("data trigger result should be returned");

        assert!(!result.is_valid());

        assert!(matches!(
            &result.errors.first().unwrap(),
            &DataTriggerError::DataTriggerConditionError(e)  if {
                e.message() == format!("Identity {owner_id} must not be equal to owner id")
            }
        ));
    }

    #[test]
    fn should_return_invalid_result_if_id_not_exists() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let mut nonce_counter = BTreeMap::new();

        let platform_state = platform.state.load();
        let mut platform_state = (**platform_state).clone();

        platform_state.set_last_committed_block_info(Some(
            ExtendedBlockInfoV0 {
                basic_info: BlockInfo {
                    time_ms: 500000,
                    height: 100,
                    core_height: 42,
                    epoch: Default::default(),
                },
                app_hash: platform.drive.grove.root_hash(None).unwrap().unwrap(),
                quorum_hash: [0u8; 32],
                block_id_hash: [0u8; 32],
                proposer_pro_tx_hash: [0u8; 32],
                signature: [0u8; 96],
                round: 0,
            }
            .into(),
        ));

        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &platform_state,
            config: &platform.config,
        };
        let protocol_version = platform_state.current_protocol_version_in_consensus();
        let platform_version = platform_state
            .current_platform_version()
            .expect("should return a platform version");

        let contact_request_document = get_contact_request_document_fixture(
            None,
            0,
            None,
            platform_state.current_protocol_version_in_consensus(),
        );
        let data_contract = get_dashpay_contract_fixture(
            None,
            0,
            platform_state.current_protocol_version_in_consensus(),
        )
        .data_contract_owned();
        let document_type = data_contract
            .document_type_for_name("contactRequest")
            .expect("expected a contact request");
        let owner_id = contact_request_document.owner_id();
        let contract_request_to_user_id = contact_request_document
            .properties()
            .get_identifier("toUserId")
            .expect("expected to get toUserId");

        let document_transitions = get_document_transitions_fixture(
            [(
                DocumentTransitionActionType::Create,
                vec![(contact_request_document, document_type, Bytes32::default())],
            )],
            &mut nonce_counter,
        );
        let document_transition = document_transitions
            .first()
            .expect("document transition should be present");

        let document_create_transition = document_transition
            .as_transition_create()
            .expect("expected a document create transition");

        let transition_execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .unwrap();

        let data_trigger_context = DataTriggerExecutionContext {
            platform: &platform_ref,
            owner_id: &owner_id,
            state_transition_execution_context: &transition_execution_context,
            transaction: None,
        };

        let _dashpay_identity_id = data_trigger_context.owner_id.to_owned();

        let result = create_contact_request_data_trigger(
            &DocumentCreateTransitionAction::from_document_borrowed_create_transition_with_contract_lookup(&platform.drive, None, document_create_transition, &BlockInfo::default(), |_identifier| {
                Ok(Arc::new(DataContractFetchInfo::dashpay_contract_fixture(protocol_version)))
            }, platform_version).expect("expected to create action").0.into(),
            &data_trigger_context,
            platform_version,
        )
        .expect("data trigger result should be returned");

        assert!(!result.is_valid());
        let data_trigger_error = &result.errors[0];

        assert!(matches!(
            data_trigger_error,
            DataTriggerError::DataTriggerConditionError(e)  if {
                e.message() == format!("Identity {contract_request_to_user_id} doesn't exist")
            }
        ));
    }
}

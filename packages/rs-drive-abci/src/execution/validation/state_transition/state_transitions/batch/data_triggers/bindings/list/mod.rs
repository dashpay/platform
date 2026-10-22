use dpp::ProtocolError;
use dpp::version::PlatformVersion;
use crate::execution::validation::state_transition::batch::data_triggers::bindings::data_trigger_binding::DataTriggerBinding;

mod v0;
mod v1;
mod v2;
mod v3;

pub fn data_trigger_bindings_list(
    platform_version: &PlatformVersion,
) -> Result<Vec<DataTriggerBinding>, ProtocolError> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .batch_state_transition
        .data_triggers
        .bindings
    {
        0 => Ok(v0::data_trigger_bindings_list_v0()?
            .into_iter()
            .map(|binding| binding.into())
            .collect()),
        1 => Ok(v1::data_trigger_bindings_list_v1()?
            .into_iter()
            .map(|binding| binding.into())
            .collect()),
        2 => Ok(v2::data_trigger_bindings_list_v2()?
            .into_iter()
            .map(|binding| binding.into())
            .collect()),
        3 => Ok(v3::data_trigger_bindings_list_v3()?
            .into_iter()
            .map(|binding| binding.into())
            .collect()),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "data_trigger_bindings".to_string(),
            known_versions: vec![0, 1, 2, 3],
            received: version,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::state_transition_execution_context::v0::StateTransitionExecutionContextV0;
    use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
    use crate::execution::validation::state_transition::batch::data_triggers::bindings::data_trigger_binding::DataTriggerBindingV0Getters;
    use crate::execution::validation::state_transition::batch::data_triggers::DataTriggerExecutionContext;
    use crate::platform_types::platform::PlatformStateRef;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::state::data_trigger::DataTriggerError;
    use dpp::identifier::Identifier;
    use dpp::system_data_contracts::token_market_contract;
    use dpp::system_data_contracts::token_market_contract::v1::document_types::{ask, bid};
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use drive::drive::contract::DataContractFetchInfo;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionV0;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{
        DocumentReplaceTransitionAction, DocumentReplaceTransitionActionV0,
    };
    use drive::state_transition_action::batch::batched_transition::document_transition::{
        DocumentTransitionAction, DocumentTransitionActionType,
    };
    use std::sync::Arc;

    /// A replace or a delete of the order `document_type_name` sent as a document transition.
    fn order_write(
        document_type_name: &str,
        action_type: DocumentTransitionActionType,
        contract: Arc<DataContractFetchInfo>,
    ) -> DocumentTransitionAction {
        let base = DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
            id: Identifier::from([9; 32]),
            identity_contract_nonce: 1,
            document_type_name: document_type_name.to_string(),
            data_contract: contract,
            token_cost: None,
            gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
            contract_gas_fees_paid_by: GasFeesPaidBy::default(),
            declared_action_fee: None,
            shielded_token_payment: None,
        });
        match action_type {
            DocumentTransitionActionType::Replace => DocumentTransitionAction::ReplaceAction(
                DocumentReplaceTransitionAction::V0(DocumentReplaceTransitionActionV0 {
                    base,
                    revision: 2,
                    created_at: None,
                    updated_at: None,
                    transferred_at: None,
                    created_at_block_height: None,
                    updated_at_block_height: None,
                    transferred_at_block_height: None,
                    created_at_core_block_height: None,
                    updated_at_core_block_height: None,
                    transferred_at_core_block_height: None,
                    data: Default::default(),
                    changed_data_fields: Default::default(),
                    removed_identifier_fields: Default::default(),
                    stored_changed_values: Default::default(),
                    creator_id: None,
                    moderated_at: None,
                    moderated_by: None,
                    property_constraint_reads: None,
                }),
            ),
            DocumentTransitionActionType::Delete => DocumentTransitionAction::DeleteAction(
                DocumentDeleteTransitionAction::V0(DocumentDeleteTransitionActionV0 { base }),
            ),
            other => panic!("not an order write: {other:?}"),
        }
    }

    /// Through the version dispatch: the latest protocol version binds exactly one trigger to
    /// each replace and delete of an `ask` and a `bid`, and running it refuses the write with
    /// `DataTriggerConditionError`. Protocol version 14 binds none of them.
    #[test]
    fn should_refuse_order_writes_through_the_latest_bindings_and_bind_none_before_15() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let state = platform.state.load();
        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &state,
            config: &platform.config,
        };
        let latest = PlatformVersion::latest();
        let contract = platform
            .drive
            .get_contract_with_fetch_info(
                token_market_contract::ID.to_buffer(),
                false,
                None,
                latest,
            )
            .expect("expected to fetch the token market contract")
            .expect("expected the token market contract at genesis");
        let order_writes = [ask::NAME, bid::NAME].into_iter().flat_map(|name| {
            [
                DocumentTransitionActionType::Replace,
                DocumentTransitionActionType::Delete,
            ]
            .into_iter()
            .map(move |action_type| (name, action_type))
        });

        let bindings = data_trigger_bindings_list(latest).expect("expected the bindings");
        for (name, action_type) in order_writes.clone() {
            let matching: Vec<&DataTriggerBinding> = bindings
                .iter()
                .filter(|binding| {
                    binding.is_matching(&token_market_contract::ID, name, action_type)
                })
                .collect();
            assert_eq!(matching.len(), 1, "{name} {action_type:?}");
            let mut execution_context =
                StateTransitionExecutionContext::V0(StateTransitionExecutionContextV0::default());
            let block_info = BlockInfo::default();
            let owner_id = Identifier::from([7; 32]);
            let mut context = DataTriggerExecutionContext {
                platform: &platform_ref,
                block_info: &block_info,
                owner_id: &owner_id,
                state_transition_execution_context: &mut execution_context,
                transaction: None,
            };
            let result = matching[0]
                .execute(
                    &order_write(name, action_type, Arc::clone(&contract)),
                    &mut context,
                    latest,
                )
                .expect("expected the trigger to run");
            assert!(
                matches!(
                    result.errors.as_slice(),
                    [DataTriggerError::DataTriggerConditionError(_)]
                ),
                "{name} {action_type:?}: {:?}",
                result.errors
            );
        }

        let before = data_trigger_bindings_list(
            PlatformVersion::get(14).expect("expected protocol version 14"),
        )
        .expect("expected the bindings");
        for (name, action_type) in order_writes {
            assert!(
                !before.iter().any(|binding| binding.is_matching(
                    &token_market_contract::ID,
                    name,
                    action_type
                )),
                "{name} {action_type:?} is bound before 15"
            );
        }
    }
}

use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::group::ModificationOfGroupActionMainParametersNotPermittedError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::GroupActionAccessors;
use dpp::prelude::Identifier;
use dpp::tokens::token_event::TokenEvent;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::token_transition::token_set_price_for_direct_purchase_transition_action::{TokenSetPriceForDirectPurchaseTransitionAction, TokenSetPriceForDirectPurchaseTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenSetPriceForDirectPurchaseTransitionActionStateValidationV0 {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl TokenSetPriceForDirectPurchaseTransitionActionStateValidationV0
    for TokenSetPriceForDirectPurchaseTransitionAction
{
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = self.base().validate_state(
            platform,
            owner_id,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // Let's first check to see if we are authorized to perform this action
        let contract = &self.data_contract_fetch_info_ref().contract;
        let token_configuration = contract.expected_token_configuration(self.token_position())?;
        let rules = token_configuration
            .distribution_rules()
            .change_direct_purchase_pricing_rules();
        let main_control_group = token_configuration.main_control_group();
        let validation_result = self.base().validate_group_action(
            rules,
            owner_id,
            contract.owner_id(),
            main_control_group,
            contract.groups(),
            "change_price_for_direct_purchase".to_string(),
            token_configuration,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        if let Some(original_group_action) = self.base().original_group_action() {
            if let GroupActionEvent::TokenEvent(TokenEvent::ChangePriceForDirectPurchase(
                maybe_token_pricing_schedule,
                _,
            )) = original_group_action.event()
            {
                let mut changed_internal_fields = vec![];
                if maybe_token_pricing_schedule.as_ref() != self.price() {
                    changed_internal_fields.push("price".to_string());
                }
                if !changed_internal_fields.is_empty() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(
                            StateError::ModificationOfGroupActionMainParametersNotPermittedError(
                                ModificationOfGroupActionMainParametersNotPermittedError::new(
                                    original_group_action.event().event_name(),
                                    "Token: directPricing".to_string(),
                                    changed_internal_fields,
                                ),
                            ),
                        ),
                    ));
                }
            } else {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(
                        StateError::ModificationOfGroupActionMainParametersNotPermittedError(
                            ModificationOfGroupActionMainParametersNotPermittedError::new(
                                original_group_action.event().event_name(),
                                "Token: directPricing".to_string(),
                                vec![],
                            ),
                        ),
                    ),
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

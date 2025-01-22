use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::{InvalidGroupPositionError, NewTokensDestinationIdentityDoesNotExistError, TokenSettingMaxSupplyToLessThanCurrentSupplyError, UnauthorizedTokenActionError};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::group::action_taker::{ActionGoal, ActionTaker};
use dpp::prelude::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::token_transition::token_config_update_transition_action::{TokenConfigUpdateTransitionAction, TokenConfigUpdateTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::error::drive::DriveError;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenConfigUpdateTransitionActionStateValidationV0 {
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
impl TokenConfigUpdateTransitionActionStateValidationV0 for TokenConfigUpdateTransitionAction {
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
        let main_control_group = token_configuration.main_control_group();

        let goal = if self.base().store_in_group().is_some() {
            // We are using a group
            // We just need to be able to participate
            ActionGoal::ActionParticipation
        } else {
            // We are not using a group
            // We need to make sure that for the change we are doing that we can finish it
            ActionGoal::ActionCompletion
        };

        if !token_configuration.can_apply_token_configuration_item(
            self.update_token_configuration_item(),
            &contract.owner_id(),
            main_control_group,
            contract.groups(),
            &ActionTaker::SingleIdentity(owner_id),
            goal,
        ) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::UnauthorizedTokenActionError(
                    UnauthorizedTokenActionError::new(
                        self.token_id(),
                        owner_id,
                        "config_update".to_string(),
                        token_configuration.authorized_action_takers_for_configuration_item(
                            self.update_token_configuration_item(),
                        ),
                    ),
                )),
            ));
        }
        
        match self.update_token_configuration_item() {
            TokenConfigurationChangeItem::MaxSupply(Some(max_supply)) => {
                // If we are setting a max supply we need to make sure it isn't less than the
                // current supply of the token
                let (token_total_supply, fee) = platform.drive.fetch_token_total_supply_with_cost(self.token_id().to_buffer(), block_info, transaction, platform_version)?;
                execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
                if let Some(token_total_supply) = token_total_supply {
                    if token_total_supply > *max_supply {
                        // We are trying to set a max supply smaller than the token total supply
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ConsensusError::StateError(StateError::TokenSettingMaxSupplyToLessThanCurrentSupplyError(
                                TokenSettingMaxSupplyToLessThanCurrentSupplyError::new(
                                    self.token_id(),
                                    *max_supply,
                                    token_total_supply,
                                ),
                            )),
                        ));
                    }
                } else {
                    return Err(Error::Drive(drive::error::Error::Drive(DriveError::CorruptedDriveState(format!("token {} total supply not found", self.token_id())))));
                }
            }
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(Some(identity_id)) => {
                // We need to make sure the identity exists
                let (identity_balance, fee) = platform.drive.fetch_identity_balance_with_costs(identity_id.to_buffer(), block_info, true, transaction, platform_version)?;
                execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
                if identity_balance.is_none() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::NewTokensDestinationIdentityDoesNotExistError(
                            NewTokensDestinationIdentityDoesNotExistError::new(
                                *identity_id
                            ),
                        )),
                    ));
                }
            }
            TokenConfigurationChangeItem::MainControlGroup(Some(control_group)) => {
                if !self.data_contract_fetch_info().contract.groups().contains_key(control_group) {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::InvalidGroupPositionError(
                            InvalidGroupPositionError::new(
                                self.data_contract_fetch_info().contract.groups().keys().last().copied().unwrap_or_default(),
                                *control_group,
                            ),
                        )),
                    ));
                }
            }
            _ => {}
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::{IdentityTokenAccountNotFrozenError, UnauthorizedTokenActionError};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::multi_identity_events::ActionTaker;
use dpp::prelude::Identifier;
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::{TokenDestroyFrozenFundsTransitionAction, TokenDestroyFrozenFundsTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenDestroyFrozenFundsTransitionActionStateValidationV0 {
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
impl TokenDestroyFrozenFundsTransitionActionStateValidationV0
    for TokenDestroyFrozenFundsTransitionAction
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

        // We need to validate that we are frozen

        let (info, fee_result) = platform.drive.fetch_identity_token_info_with_costs(
            self.token_id().to_buffer(),
            self.frozen_identity_id().to_buffer(),
            block_info,
            true,
            transaction,
            platform_version,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

        if info.is_none() || !info.unwrap().frozen() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::IdentityTokenAccountNotFrozenError(
                    IdentityTokenAccountNotFrozenError::new(
                        self.token_id(),
                        self.frozen_identity_id(),
                        "destroy_frozen_funds".to_string(),
                    ),
                )),
            ));
        }

        // Let's first check to see if we are authorized to perform this action
        let contract = &self.data_contract_fetch_info_ref().contract;
        let token_configuration = contract.expected_token_configuration(self.token_position())?;
        let rules = token_configuration.destroy_frozen_funds_rules();
        let main_control_group = token_configuration
            .main_control_group()
            .map(|position| contract.expected_group(position))
            .transpose()?;

        if !rules.can_make_change(
            &contract.owner_id(),
            main_control_group,
            contract.groups(),
            &ActionTaker::SingleIdentity(owner_id),
        ) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::UnauthorizedTokenActionError(
                    UnauthorizedTokenActionError::new(
                        self.token_id(),
                        owner_id,
                        "destroy_frozen_funds".to_string(),
                        rules.authorized_to_make_change_action_takers().clone(),
                    ),
                )),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

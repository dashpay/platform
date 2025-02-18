use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::IdentityTokenAccountAlreadyFrozenError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::prelude::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::token_transition::token_freeze_transition_action::{TokenFreezeTransitionAction, TokenFreezeTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;

use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
use dpp::consensus::ConsensusError;
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenFreezeTransitionActionStateValidationV0 {
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

impl TokenFreezeTransitionActionStateValidationV0 for TokenFreezeTransitionAction {
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
        let rules = token_configuration.freeze_rules();
        let main_control_group = token_configuration.main_control_group();
        let validation_result = self.base().validate_group_action(
            rules,
            owner_id,
            contract.owner_id(),
            main_control_group,
            contract.groups(),
            "freeze".to_string(),
            token_configuration,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // Check if the identity is already frozen
        let (info, fee_result) = platform.drive.fetch_identity_token_info_with_costs(
            self.token_id().to_buffer(),
            self.identity_to_freeze_id().to_buffer(),
            block_info,
            true,
            transaction,
            platform_version,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee_result));
        if let Some(info) = info {
            if info.frozen() == true {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::IdentityTokenAccountAlreadyFrozenError(
                        IdentityTokenAccountAlreadyFrozenError::new(
                            self.token_id(),
                            owner_id,
                            "Freeze Identity Token Account".to_string(),
                        ),
                    )),
                ));
            }
        };

        Ok(SimpleConsensusValidationResult::new())
    }
}

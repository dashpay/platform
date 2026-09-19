use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
use crate::execution::validation::state_transition::batch::action_validation::token::token_shielded_pool_common::{
    validate_token_shielded_pool_enabled, verify_token_pool_bundle,
};
use crate::execution::validation::state_transition::state_transitions::shielded_common::FLAGS_OUTPUTS_ONLY;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::TokenMintPastMaxSupplyError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use drive::error::drive::DriveError;
use drive::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_to_pool_transition_action::{
    TokenDirectPurchaseToPoolTransitionAction, TokenDirectPurchaseToPoolTransitionActionAccessorsV0,
};

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenDirectPurchaseToPoolTransitionActionStateValidationV0 {
    #[allow(clippy::too_many_arguments)]
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        validation_mode: ValidationMode,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl TokenDirectPurchaseToPoolTransitionActionStateValidationV0
    for TokenDirectPurchaseToPoolTransitionAction
{
    /// The price was checked when the action was built (as for a transparent purchase); here the
    /// max supply is checked and the outputs-only bundle must carry exactly `token_count`.
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        validation_mode: ValidationMode,
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

        let validation_result = validate_token_shielded_pool_enabled(self.base())?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let token_id = self.token_id();

        let contract = &self.data_contract_fetch_info_ref().contract;
        let token_configuration = contract.expected_token_configuration(self.token_position())?;

        if let Some(max_supply) = token_configuration.max_supply() {
            let (token_total_supply, fee) = platform.drive.fetch_token_total_supply_with_cost(
                token_id.to_buffer(),
                block_info,
                transaction,
                platform_version,
            )?;
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
            let Some(token_total_supply) = token_total_supply else {
                return Err(Error::Drive(drive::error::Error::Drive(
                    DriveError::CorruptedDriveState(format!(
                        "token {} total supply not found",
                        token_id
                    )),
                )));
            };
            match token_total_supply.checked_add(self.token_count()) {
                Some(total_supply_after) if total_supply_after <= max_supply => {}
                _ => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(
                            TokenMintPastMaxSupplyError::new(
                                token_id,
                                self.token_count(),
                                token_total_supply,
                                max_supply,
                            ),
                        )),
                    ));
                }
            }
        }

        verify_token_pool_bundle(
            validation_mode,
            self.actions(),
            FLAGS_OUTPUTS_ONLY,
            -(self.token_count() as i64),
            self.anchor(),
            self.proof(),
            self.binding_signature(),
            &[],
        )
    }
}

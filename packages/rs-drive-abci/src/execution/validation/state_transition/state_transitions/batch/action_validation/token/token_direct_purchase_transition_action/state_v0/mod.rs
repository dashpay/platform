use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::{IdentityTokenAccountFrozenError, TokenIsPausedError, TokenMintPastMaxSupplyError};
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::prelude::Identifier;
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::tokens::status::v0::TokenStatusV0Accessors;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_transition_action::{TokenDirectPurchaseTransitionAction, TokenDirectPurchaseTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::error::drive::DriveError;
use drive::query::TransactionArg;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenDirectPurchaseTransitionActionStateValidationV0 {
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
impl TokenDirectPurchaseTransitionActionStateValidationV0 for TokenDirectPurchaseTransitionAction {
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

        // We need to verify that the token is not paused
        let (token_status, fee_result) = platform.drive.fetch_token_status_with_costs(
            self.token_id().to_buffer(),
            block_info,
            true,
            transaction,
            platform_version,
        )?;

        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

        if let Some(status) = token_status {
            if status.paused() {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::TokenIsPausedError(
                        TokenIsPausedError::new(self.token_id()),
                    )),
                ));
            }
        }

        // We need to verify that the buyer's token account is not frozen
        let (info, fee_result) = platform.drive.fetch_identity_token_info_with_costs(
            self.token_id().to_buffer(),
            owner_id.to_buffer(),
            block_info,
            true,
            transaction,
            platform_version,
        )?;

        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

        if let Some(info) = info {
            if info.frozen() {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::IdentityTokenAccountFrozenError(
                        IdentityTokenAccountFrozenError::new(
                            self.token_id(),
                            owner_id,
                            "direct_purchase".to_string(),
                        ),
                    )),
                ));
            }
        };

        let contract = &self.data_contract_fetch_info_ref().contract;
        let token_configuration = contract.expected_token_configuration(self.token_position())?;

        if let Some(max_supply) = token_configuration.max_supply() {
            // We have a max supply, let's get the current supply
            let (token_total_supply, fee) = platform.drive.fetch_token_total_supply_with_cost(
                self.token_id().to_buffer(),
                block_info,
                transaction,
                platform_version,
            )?;
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
            if let Some(token_total_supply) = token_total_supply {
                if let Some(total_supply_after_direct_purchase) =
                    token_total_supply.checked_add(self.token_count())
                {
                    if total_supply_after_direct_purchase > max_supply {
                        // By purchasing we are adding past the token total supply
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(
                                TokenMintPastMaxSupplyError::new(
                                    self.token_id(),
                                    self.token_count(),
                                    token_total_supply,
                                    max_supply,
                                ),
                            )),
                        ));
                    }
                } else {
                    // if we overflow we would also always go over max supply
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(
                            TokenMintPastMaxSupplyError::new(
                                self.token_id(),
                                self.token_count(),
                                token_total_supply,
                                max_supply,
                            ),
                        )),
                    ));
                }
            } else {
                return Err(Error::Drive(drive::error::Error::Drive(
                    DriveError::CorruptedDriveState(format!(
                        "token {} total supply not found",
                        self.token_id()
                    )),
                )));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

use dpp::block::block_info::BlockInfo;
use dpp::errors::consensus::ConsensusError;
use dpp::errors::consensus::state::state_error::StateError;
use dpp::errors::consensus::state::token::{IdentityDoesNotHaveEnoughTokenBalanceError, IdentityTokenAccountFrozenError, TokenIsPausedError};
use dpp::prelude::Identifier;
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::tokens::status::v0::TokenStatusV0Accessors;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::TokenTransferTransitionAction;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::v0::TokenTransferTransitionActionAccessorsV0;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenTransferTransitionActionStateValidationV0 {
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
impl TokenTransferTransitionActionStateValidationV0 for TokenTransferTransitionAction {
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

        // We need to verify that we have enough of the token
        let balance = platform
            .drive
            .fetch_identity_token_balance(
                self.token_id().to_buffer(),
                owner_id.to_buffer(),
                transaction,
                platform_version,
            )?
            .unwrap_or_default();
        execution_context.add_operation(ValidationOperation::RetrieveIdentityTokenBalance);
        if balance < self.amount() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::IdentityDoesNotHaveEnoughTokenBalanceError(
                    IdentityDoesNotHaveEnoughTokenBalanceError::new(
                        self.token_id(),
                        owner_id,
                        self.amount(),
                        balance,
                        "transfer".to_string(),
                    ),
                )),
            ));
        }

        // We need to verify that our token account is not frozen
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
                            "transfer".to_string(),
                        ),
                    )),
                ));
            }
        };

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

        Ok(SimpleConsensusValidationResult::new())
    }
}

use dpp::block::block_info::BlockInfo;
use dpp::prelude::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_place_ask_transition_action::{TokenPlaceAskTransitionAction, TokenPlaceAskTransitionActionAccessorsV0};
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::IdentityDoesNotHaveEnoughTokenBalanceError;
use dpp::consensus::ConsensusError;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
use crate::execution::validation::state_transition::batch::action_validation::token::token_order_common::validate_token_account_not_frozen;
use crate::execution::validation::state_transition::batch::action_validation::token::token_shielded_pool_common::validate_token_not_paused;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenPlaceAskTransitionActionStateValidationV0 {
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

impl TokenPlaceAskTransitionActionStateValidationV0 for TokenPlaceAskTransitionAction {
    /// A listing needs a token that is not paused and a seller whose account is not frozen and
    /// who holds the amount. What needs no read (a tradeable token, a total that fits in an
    /// order, an expiry after the block) is checked with the batch's advanced structure.
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

        let validation_result = validate_token_not_paused(
            platform,
            self.token_id(),
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let validation_result = validate_token_account_not_frozen(
            platform,
            self.token_id(),
            owner_id,
            "placeAsk",
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

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
                        "placeAsk".to_string(),
                    ),
                )),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

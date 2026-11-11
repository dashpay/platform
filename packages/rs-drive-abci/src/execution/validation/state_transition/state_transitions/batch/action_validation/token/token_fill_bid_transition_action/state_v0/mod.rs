use dpp::block::block_info::BlockInfo;
use dpp::prelude::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_fill_bid_transition_action::{TokenFillBidTransitionAction, TokenFillBidTransitionActionAccessorsV0};
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::IdentityDoesNotHaveEnoughTokenBalanceError;
use dpp::consensus::ConsensusError;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::execution::validation::state_transition::batch::action_validation::token::token_order_common::validate_token_account_not_frozen;
use crate::execution::validation::state_transition::batch::action_validation::token::token_shielded_pool_common::validate_token_not_paused;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenFillBidTransitionActionStateValidationV0 {
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

impl TokenFillBidTransitionActionStateValidationV0 for TokenFillBidTransitionAction {
    /// The transformer already read the offer and refused a fill it cannot meet, and the batch's
    /// advanced structure a token that is no longer tradeable. A sale also needs a token that is
    /// not paused, a seller whose account is not frozen and who holds the amount the fill takes,
    /// and a buyer whose account is not frozen unless the token allows transfers to frozen
    /// balances.
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
            "fillBid",
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        if !self
            .base()
            .token_configuration()?
            .is_allowed_transfer_to_frozen_balance()
        {
            let validation_result = validate_token_account_not_frozen(
                platform,
                self.token_id(),
                self.order().maker_id,
                "fillBid",
                block_info,
                execution_context,
                transaction,
                platform_version,
            )?;
            if !validation_result.is_valid() {
                return Ok(validation_result);
            }
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

        if balance < self.fill_amount() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::IdentityDoesNotHaveEnoughTokenBalanceError(
                    IdentityDoesNotHaveEnoughTokenBalanceError::new(
                        self.token_id(),
                        owner_id,
                        self.fill_amount(),
                        balance,
                        "fillBid".to_string(),
                    ),
                )),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

use dpp::block::block_info::BlockInfo;
use dpp::prelude::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_fill_ask_transition_action::{TokenFillAskTransitionAction, TokenFillAskTransitionActionAccessorsV0};
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::execution::validation::state_transition::batch::action_validation::token::token_order_common::{validate_token_account_not_frozen, validate_token_tradeable};
use crate::execution::validation::state_transition::batch::action_validation::token::token_shielded_pool_common::validate_token_not_paused;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenFillAskTransitionActionStateValidationV0 {
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

impl TokenFillAskTransitionActionStateValidationV0 for TokenFillAskTransitionAction {
    /// The transformer already read the listing and refused a fill it cannot meet. A fill also
    /// needs a token that is still tradeable and not paused, a seller whose account is not
    /// frozen, and a buyer whose account is not frozen unless the token allows transfers to
    /// frozen balances. The buyer's credit balance is checked with the batch's fees, which count
    /// the fill's cost as a purchase.
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

        let validation_result = validate_token_tradeable(self.base())?;
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
            self.order().maker_id,
            "fillAsk",
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
                owner_id,
                "fillAsk",
                block_info,
                execution_context,
                transaction,
                platform_version,
            )?;
            if !validation_result.is_valid() {
                return Ok(validation_result);
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

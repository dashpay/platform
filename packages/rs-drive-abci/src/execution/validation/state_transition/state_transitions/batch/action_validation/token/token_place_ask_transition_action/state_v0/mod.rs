use dpp::block::block_info::BlockInfo;
use dpp::prelude::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_place_ask_transition_action::{TokenPlaceAskTransitionAction, TokenPlaceAskTransitionActionAccessorsV0};
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::{IdentityDoesNotHaveEnoughTokenBalanceError, TokenOrderExpiredError, TokenOrderValueTooLargeError};
use dpp::consensus::ConsensusError;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration_convention::accessors::v0::TokenConfigurationConventionV0Getters;
use dpp::tokens::token_order::TokenOrder;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
use crate::execution::validation::state_transition::batch::action_validation::token::token_order_common::{validate_token_account_not_frozen, validate_token_tradeable};
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
    /// A listing needs a tradeable token that is not paused, a seller whose account is not
    /// frozen and who holds the amount, a total that fits in an order (`i64::MAX` credits) and,
    /// when it expires, an expiry after the block.
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

        let decimals = self.base().token_configuration()?.conventions().decimals();
        if TokenOrder::order_value(self.amount(), self.price(), decimals).is_none() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                TokenOrderValueTooLargeError::new(self.amount(), self.price(), decimals).into(),
            ));
        }

        if let Some(expires_at) = self.expires_at() {
            if expires_at <= block_info.time_ms {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    TokenOrderExpiredError::new(self.order_id(), expires_at, block_info.time_ms)
                        .into(),
                ));
            }
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

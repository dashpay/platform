use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_shielded_pool_common::{
    validate_token_not_paused, validate_token_pool_output_nullifiers,
    validate_token_shielded_pool_enabled, verify_token_pool_bundle,
};
use crate::execution::validation::state_transition::state_transitions::shielded_common::FLAGS_OUTPUTS_ONLY;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::IdentityDoesNotHaveEnoughTokenBalanceError;
use dpp::consensus::ConsensusError;
use dpp::prelude::Identifier;
use dpp::shielded::token_pool_output_only_extra_sighash_data;
use dpp::state_transition::batch_transition::batched_transition::token_transition_action_type::TokenTransitionActionType;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_shield_transition_action::{
    TokenShieldTransitionAction, TokenShieldTransitionActionAccessorsV0,
};

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenShieldTransitionActionStateValidationV0 {
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

impl TokenShieldTransitionActionStateValidationV0 for TokenShieldTransitionAction {
    /// Shielding is a transfer from the owner's balance into the pool, so it runs the sender
    /// side of the transfer checks (balance, paused token) on top of the pool opt-in; a pooled
    /// token can never freeze an account, so no frozen check is read or billed. It then
    /// verifies the outputs-only bundle.
    ///
    /// Historical note: the pool
    /// opt-in, then verifies the outputs-only bundle. The bundle's anchor is not checked
    /// against the pool: with spends disabled the anchor is not consumed.
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

        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
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
                        token_id,
                        owner_id,
                        self.amount(),
                        balance,
                        "shield".to_string(),
                    ),
                )),
            ));
        }

        let validation_result = validate_token_not_paused(
            platform,
            token_id,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let validation_result = validate_token_pool_output_nullifiers(
            platform,
            &token_id.to_buffer(),
            &self.nullifiers(),
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // Outputs-only bundle entering the pool: value balance is `-amount`.
        // An outputs-only bundle has no anchor of its own, so the sighash is what pins it to
        // this pool and this kind — see `token_pool_output_only_extra_sighash_data`.
        let extra_sighash_data = token_pool_output_only_extra_sighash_data(
            TokenTransitionActionType::Shield,
            token_id.as_bytes(),
            platform_version,
        )?;
        verify_token_pool_bundle(
            validation_mode,
            self.actions(),
            FLAGS_OUTPUTS_ONLY,
            -(self.amount() as i64),
            self.anchor(),
            self.proof(),
            self.binding_signature(),
            &extra_sighash_data,
        )
    }
}

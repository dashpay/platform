use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_shielded_pool_common::{
    charge_drive_operations, validate_token_not_paused, validate_token_pool_anchor_exists,
    validate_token_pool_nullifiers, validate_token_shielded_pool_enabled,
    verify_token_pool_bundle,
};
use crate::execution::validation::state_transition::state_transitions::shielded_common::FLAGS_SPENDS_AND_OUTPUTS;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::Identifier;
use dpp::shielded::token_shielded_transfer_extra_sighash_data;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_shielded_transfer_transition_action::{
    TokenShieldedTransferTransitionAction, TokenShieldedTransferTransitionActionAccessorsV0,
};

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenShieldedTransferTransitionActionStateValidationV0 {
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

impl TokenShieldedTransferTransitionActionStateValidationV0
    for TokenShieldedTransferTransitionAction
{
    /// A pool-internal transfer touches no identity balance: pool opt-in, paused token, anchor
    /// and unspent nullifiers, then the zero-value-balance spend bundle is verified with the
    /// token and batch owner bound into the sighash. No notes floor applies: nothing leaves
    /// the pool, exactly as for the credit pool's `ShieldedTransfer`.
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

        let token_id_bytes = token_id.to_buffer();
        let mut drive_operations = vec![];

        let validation_result = validate_token_pool_anchor_exists(
            platform.drive,
            &token_id_bytes,
            self.anchor(),
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            charge_drive_operations(
                platform.drive,
                block_info,
                execution_context,
                drive_operations,
                platform_version,
            )?;
            return Ok(validation_result);
        }

        let validation_result = validate_token_pool_nullifiers(
            platform.drive,
            &token_id_bytes,
            &self.nullifiers(),
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        charge_drive_operations(
            platform.drive,
            block_info,
            execution_context,
            drive_operations,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let extra_sighash_data = token_shielded_transfer_extra_sighash_data(
            &token_id_bytes,
            &owner_id.to_buffer(),
            platform_version,
        )?;

        verify_token_pool_bundle(
            validation_mode,
            self.actions(),
            FLAGS_SPENDS_AND_OUTPUTS,
            0,
            self.anchor(),
            self.proof(),
            self.binding_signature(),
            &extra_sighash_data,
        )
    }
}

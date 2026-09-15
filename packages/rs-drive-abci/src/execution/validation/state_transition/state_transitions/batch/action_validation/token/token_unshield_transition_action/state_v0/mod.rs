use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_shielded_pool_common::{
    charge_drive_operations, validate_identity_token_account_not_frozen,
    validate_minimum_token_pool_notes, validate_token_not_paused, validate_token_pool_anchor_exists,
    validate_token_pool_nullifiers, validate_token_shielded_pool_enabled,
    verify_token_pool_bundle,
};
use crate::execution::validation::state_transition::common::validate_identity_exists::validate_identity_exists;
use crate::execution::validation::state_transition::state_transitions::shielded_common::FLAGS_SPENDS_AND_OUTPUTS;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::TokenTransferRecipientIdentityNotExistError;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::prelude::Identifier;
use dpp::shielded::token_unshield_extra_sighash_data;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_unshield_transition_action::{
    TokenUnshieldTransitionAction, TokenUnshieldTransitionActionAccessorsV0,
};

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenUnshieldTransitionActionStateValidationV0 {
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

impl TokenUnshieldTransitionActionStateValidationV0 for TokenUnshieldTransitionAction {
    /// Unshielding is a transfer from the pool into the recipient's balance: the recipient
    /// side of the transfer checks (exists, frozen account unless allowed, paused token) plus
    /// the pool-side checks the credit pool's `Unshield` runs (notes floor, anchor, unspent
    /// nullifiers, pool balance), then the spend bundle is verified with the token, batch
    /// owner, recipient and amount bound into the sighash.
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
        let recipient_id = self.recipient_id();

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

        let recipient_exists = validate_identity_exists(
            platform.drive,
            &recipient_id,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !recipient_exists {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                TokenTransferRecipientIdentityNotExistError::new(recipient_id).into(),
            ));
        }

        if !self
            .base()
            .token_configuration()?
            .is_allowed_transfer_to_frozen_balance()
        {
            let validation_result = validate_identity_token_account_not_frozen(
                platform,
                token_id,
                recipient_id,
                "unshield",
                block_info,
                execution_context,
                transaction,
                platform_version,
            )?;
            if !validation_result.is_valid() {
                return Ok(validation_result);
            }
        }

        let token_id_bytes = token_id.to_buffer();
        let mut drive_operations = vec![];

        let validation_result = validate_minimum_token_pool_notes(
            platform.drive,
            &token_id_bytes,
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

        let pool_balance = platform.drive.read_token_shielded_pool_total_balance(
            &token_id_bytes,
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

        if pool_balance < self.amount() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                StateError::InvalidShieldedProofError(InvalidShieldedProofError::new(format!(
                    "token shielded pool has insufficient balance: pool has {} but unshield requires {}",
                    pool_balance,
                    self.amount()
                )))
                .into(),
            ));
        }

        let extra_sighash_data = token_unshield_extra_sighash_data(
            &token_id_bytes,
            &owner_id.to_buffer(),
            &recipient_id.to_buffer(),
            self.amount(),
            platform_version,
        )?;

        verify_token_pool_bundle(
            execution_context,
            validation_mode,
            self.actions(),
            FLAGS_SPENDS_AND_OUTPUTS,
            self.amount() as i64,
            self.anchor(),
            self.proof(),
            self.binding_signature(),
            &extra_sighash_data,
            platform_version,
        )
    }
}

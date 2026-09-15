//! Stateful checks shared by the three token shielded pool validators (`TokenShield`,
//! `TokenUnshield`, `TokenShieldedTransfer`).
//!
//! These mirror the credit pool helpers in `shielded_common`, re-rooted under the token's
//! pool, and meter every read into the execution context so the batch owner pays for them.
//! The Orchard proof is verified last and only in block processing (`ValidationMode::Validator`);
//! CheckTx runs it separately under the nonce-aware `CheckTxProofVerifier` admission, exactly
//! as `ShieldFromIdentity` does, so a mempool flood of bad proofs cannot burn validator CPU
//! for free while a bad proof in a block is still a paid failure (the identity contract nonce
//! is consumed and the verification fee charged).

use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::state_transitions::shielded_common::reconstruct_and_verify_bundle;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::shielded::insufficient_pool_notes_error::InsufficientPoolNotesError;
use dpp::consensus::state::shielded::invalid_anchor_error::InvalidAnchorError;
use dpp::consensus::state::shielded::nullifier_already_spent_error::NullifierAlreadySpentError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::{
    IdentityTokenAccountFrozenError, TokenIsPausedError, TokenShieldedPoolNotEnabledError,
};
use dpp::consensus::ConsensusError;
use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::Identifier;
use dpp::shielded::{compute_shielded_verification_fee, SerializedAction};
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::tokens::status::v0::TokenStatusV0Accessors;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::fees::op::LowLevelDriveOperation;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
    TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0,
};
use std::collections::HashSet;

/// The token's configuration must opt into a shielded pool.
pub(super) fn validate_token_shielded_pool_enabled(
    base: &TokenBaseTransitionAction,
) -> Result<SimpleConsensusValidationResult, Error> {
    if !base.token_configuration()?.has_shielded_pool() {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            TokenShieldedPoolNotEnabledError::new(base.token_id()).into(),
        ));
    }
    Ok(SimpleConsensusValidationResult::new())
}

/// A paused token allows no shielded operation either: shielding, unshielding and pool
/// transfers all move the token.
pub(super) fn validate_token_not_paused(
    platform: &PlatformStateRef,
    token_id: Identifier,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let (token_status, fee_result) = platform.drive.fetch_token_status_with_costs(
        token_id.to_buffer(),
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
                    TokenIsPausedError::new(token_id),
                )),
            ));
        }
    }

    Ok(SimpleConsensusValidationResult::new())
}

/// A frozen identity token account can neither shield out of nor (unless the token allows
/// transfers to frozen balances) receive an unshield into its balance.
#[allow(clippy::too_many_arguments)]
pub(super) fn validate_identity_token_account_not_frozen(
    platform: &PlatformStateRef,
    token_id: Identifier,
    identity_id: Identifier,
    action_name: &str,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let (info, fee_result) = platform.drive.fetch_identity_token_info_with_costs(
        token_id.to_buffer(),
        identity_id.to_buffer(),
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
                        token_id,
                        identity_id,
                        action_name.to_string(),
                    ),
                )),
            ));
        }
    }

    Ok(SimpleConsensusValidationResult::new())
}

/// Meters the GroveDB reads collected in `drive_operations` into the execution context so
/// the batch owner pays for them, as every other identity-paid read does.
pub(super) fn charge_drive_operations(
    drive: &Drive,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    drive_operations: Vec<LowLevelDriveOperation>,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    if drive_operations.is_empty() {
        return Ok(());
    }
    let fee_result = Drive::calculate_fee(
        None,
        Some(drive_operations),
        &block_info.epoch,
        drive.config.epochs_per_era,
        platform_version,
        None,
    )?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee_result));
    Ok(())
}

/// The bundle's anchor must be one of the token pool's recorded anchors (O(1) lookup).
pub(super) fn validate_token_pool_anchor_exists(
    drive: &Drive,
    token_id: &[u8; 32],
    anchor: &[u8; 32],
    transaction: TransactionArg,
    drive_operations: &mut Vec<LowLevelDriveOperation>,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let found = drive.has_token_pool_anchor(
        token_id,
        anchor,
        transaction,
        drive_operations,
        platform_version,
    )?;
    if !found {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            StateError::InvalidAnchorError(InvalidAnchorError::new(*anchor)).into(),
        ));
    }
    Ok(SimpleConsensusValidationResult::new())
}

/// No nullifier may repeat within the bundle or already be spent in the token pool.
pub(super) fn validate_token_pool_nullifiers(
    drive: &Drive,
    token_id: &[u8; 32],
    nullifiers: &[[u8; 32]],
    transaction: TransactionArg,
    drive_operations: &mut Vec<LowLevelDriveOperation>,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let mut seen = HashSet::new();
    for nullifier in nullifiers {
        if !seen.insert(nullifier) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                StateError::NullifierAlreadySpentError(NullifierAlreadySpentError::new(*nullifier))
                    .into(),
            ));
        }
    }
    for nullifier in nullifiers {
        if drive.has_token_pool_nullifier(
            token_id,
            nullifier,
            transaction,
            drive_operations,
            platform_version,
        )? {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                StateError::NullifierAlreadySpentError(NullifierAlreadySpentError::new(*nullifier))
                    .into(),
            ));
        }
    }
    Ok(SimpleConsensusValidationResult::new())
}

/// The anonymity-set floor for outflows with an observable destination
/// (`minimum_token_pool_notes_for_outgoing`, 0 at introduction).
pub(super) fn validate_minimum_token_pool_notes(
    drive: &Drive,
    token_id: &[u8; 32],
    transaction: TransactionArg,
    drive_operations: &mut Vec<LowLevelDriveOperation>,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let min_notes = platform_version
        .drive_abci
        .validation_and_processing
        .event_constants
        .minimum_token_pool_notes_for_outgoing;
    if min_notes > 0 {
        let notes_count = drive.token_shielded_pool_notes_count(
            token_id,
            transaction,
            drive_operations,
            platform_version,
        )?;
        if notes_count < min_notes {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                StateError::InsufficientPoolNotesError(InsufficientPoolNotesError::new(
                    notes_count,
                    min_notes,
                ))
                .into(),
            ));
        }
    }
    Ok(SimpleConsensusValidationResult::new())
}

/// Verifies the Orchard bundle as a PAID step.
///
/// The flat shielded compute fee (`compute_shielded_verification_fee`: one bundle verification
/// plus the per-action work) is charged BEFORE verifying, so an invalid proof in a block still
/// costs its verifier time and consumes the identity contract nonce. In CheckTx the proof is
/// skipped here: `check_tx` verifies it afterwards under the nonce-aware admission limiter,
/// once the fee estimate has shown the identity can pay for it.
#[allow(clippy::too_many_arguments)]
pub(super) fn verify_token_pool_bundle(
    execution_context: &mut StateTransitionExecutionContext,
    validation_mode: ValidationMode,
    actions: &[SerializedAction],
    flags: u8,
    value_balance: i64,
    anchor: &[u8; 32],
    proof: &[u8],
    binding_signature: &[u8; 64],
    extra_sighash_data: &[u8],
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let verification_fee = compute_shielded_verification_fee(actions.len(), platform_version)?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(FeeResult {
        processing_fee: verification_fee,
        ..Default::default()
    }));

    if !matches!(validation_mode, ValidationMode::Validator) {
        return Ok(SimpleConsensusValidationResult::new());
    }

    if let Err(error) = reconstruct_and_verify_bundle(
        actions,
        flags,
        value_balance,
        anchor,
        proof,
        binding_signature,
        extra_sighash_data,
    ) {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            StateError::InvalidShieldedProofError(error).into(),
        ));
    }

    Ok(SimpleConsensusValidationResult::new())
}

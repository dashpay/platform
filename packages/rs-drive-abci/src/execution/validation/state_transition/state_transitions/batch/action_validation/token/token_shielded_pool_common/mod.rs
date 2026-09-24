//! Stateful checks shared by the token shielded pool validators.
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
use dpp::consensus::state::token::{TokenIsPausedError, TokenShieldedPoolNotEnabledError};
use dpp::consensus::ConsensusError;
use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Getters;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::prelude::Identifier;
use dpp::shielded::{SerializedAction};
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
pub(crate) fn validate_token_shielded_pool_enabled(
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
pub(crate) fn validate_token_not_paused(
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

/// Meters the GroveDB reads collected in `drive_operations` into the execution context so
/// the batch owner pays for them, as every other identity-paid read does.
pub(crate) fn charge_drive_operations(
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
pub(crate) fn validate_token_pool_anchor_exists(
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
pub(crate) fn validate_token_pool_nullifiers(
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

/// An outputs-only bundle's dummy nullifiers must not repeat within the bundle or already be
/// recorded in the token pool. The reads are charged to the batch owner.
///
/// Such a bundle binds no owner and no anchor, so the same authorized bytes can be submitted
/// again, by anybody, as the same kind into the same pool. Each of its notes takes its `rho`
/// from its action's dummy nullifier, so the copy would land a second note with the same
/// commitment and the same nullifier, of which only one could ever be spent. Every token pool
/// write that takes an outputs-only bundle records these nullifiers; this is the check that
/// makes the record refuse anything, since the insert itself does not look for an existing
/// entry.
#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_token_pool_output_nullifiers(
    platform: &PlatformStateRef,
    token_id: &[u8; 32],
    nullifiers: &[[u8; 32]],
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let mut drive_operations = vec![];
    let validation_result = validate_token_pool_nullifiers(
        platform.drive,
        token_id,
        nullifiers,
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
    Ok(validation_result)
}

/// The token's configured notes threshold for outflows with an observable destination
/// (`minimumPoolNotesForOutgoing`, none unless the configuration sets one). The pool's notes
/// are counted only when there is a threshold to compare them with.
pub(crate) fn validate_minimum_token_pool_notes(
    drive: &Drive,
    token_id: &[u8; 32],
    token_configuration: &TokenConfiguration,
    transaction: TransactionArg,
    drive_operations: &mut Vec<LowLevelDriveOperation>,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let min_notes = token_configuration.minimum_pool_notes_for_outgoing();
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
pub(crate) fn verify_token_pool_bundle(
    validation_mode: ValidationMode,
    actions: &[SerializedAction],
    flags: u8,
    value_balance: i64,
    anchor: &[u8; 32],
    proof: &[u8],
    binding_signature: &[u8; 64],
    extra_sighash_data: &[u8],
) -> Result<SimpleConsensusValidationResult, Error> {
    // The verification fee was charged when the action was built (see the token pool action
    // transformers), so CheckTx admission and block execution price it once and identically.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::setup_platform;
    use assert_matches::assert_matches;

    const TOKEN_ID: [u8; 32] = [7u8; 32];

    /// A bundle naming one nullifier in two actions would land two notes of which only one
    /// could be spent, and an outputs-only bundle's pool write would insert that nullifier
    /// twice in one batch. The pool here has recorded nothing, so only the within-bundle
    /// check can refuse it.
    #[test]
    fn a_nullifier_repeated_within_one_bundle_is_refused_by_a_pool_that_has_not_seen_it() {
        let platform = setup_platform();
        let platform_version = PlatformVersion::latest();
        let operations = platform
            .drive
            .create_token_shielded_pool_trees_operations(
                TOKEN_ID,
                false,
                &mut None,
                None,
                platform_version,
            )
            .expect("pool tree operations");
        platform
            .drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("create pool trees");

        let nullifier = [1u8; 32];
        let result = validate_token_pool_nullifiers(
            &platform.drive,
            &TOKEN_ID,
            &[nullifier, nullifier],
            None,
            &mut vec![],
            platform_version,
        )
        .expect("no execution error");
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::NullifierAlreadySpentError(error))]
                if error.nullifier() == nullifier
        );

        // Named once, the same nullifier is fresh: what was refused is the repeat.
        let result = validate_token_pool_nullifiers(
            &platform.drive,
            &TOKEN_ID,
            &[nullifier],
            None,
            &mut vec![],
            platform_version,
        )
        .expect("no execution error");
        assert!(result.is_valid());
    }
}

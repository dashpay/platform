use crate::error::Error;
use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::state_transitions::shielded_common::{
    read_pool_total_balance, validate_anchor_exists, validate_minimum_pool_notes,
    validate_nullifiers,
};
use dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
use dpp::consensus::state::state_error::StateError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::identity_create_from_shielded_pool::IdentityCreateFromShieldedPoolTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create_from_shielded_pool) trait IdentityCreateFromShieldedPoolStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreateFromShieldedPoolStateTransitionTransformIntoActionValidationV0
    for IdentityCreateFromShieldedPoolTransition
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let IdentityCreateFromShieldedPoolTransition::V0(v0) = self;

        let anchor: [u8; 32] = v0.anchor;
        let nullifiers: Vec<[u8; 32]> = v0.actions.iter().map(|a| a.nullifier).collect();

        // The (stateless) key structure, per-key proof-of-possession, denomination membership, and
        // id re-derivation are all validated earlier — basic structure (`validate_structure`) and
        // `validate_shielded_proof` (the latter runs the PoP + key structure BEFORE Halo 2 so a
        // malformed PoP cannot make the node pay for proof verification). Here we only do the
        // STATEFUL checks against the shielded pool, then account for the per-key PoP verifications.

        // Read the current shielded pool state (read-your-own-writes within the block transaction).
        let mut drive_operations = vec![];
        let current_total_balance =
            read_pool_total_balance(drive, transaction, &mut drive_operations, platform_version)?;

        // Minimum-notes anonymity-set threshold for outgoing transitions.
        if let Some(consensus_error) = validate_minimum_pool_notes(
            drive,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(consensus_error);
        }

        // The anchor must exist in the recorded anchors tree.
        if let Some(consensus_error) = validate_anchor_exists(
            drive,
            &anchor,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(consensus_error);
        }

        // Nullifiers must be unspent in state and not duplicated intra-bundle (read-your-own-writes).
        if let Some(consensus_error) = validate_nullifiers(
            drive,
            &nullifiers,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(consensus_error);
        }

        // The pool must hold at least the full denomination leaving it.
        if current_total_balance < v0.denomination {
            return Ok(ConsensusValidationResult::new_with_error(
                StateError::InvalidShieldedProofError(InvalidShieldedProofError::new(format!(
                    "shielded pool has insufficient balance: pool has {} but identity-create exit requires {}",
                    current_total_balance, v0.denomination
                )))
                .into(),
            ));
        }

        // The identity-creation state checks (the new identity must not already exist, and none of
        // its public-key hashes may already be registered to another identity) are NOT done here.
        // They live in `validate_state`, which branches the outcome: it forwards the success action
        // built below when the checks pass, or returns an `UnshieldAction` that finalizes the spend
        // and credits the fallback address minus a penalty when the unique-key-hash check fails
        // (mirroring `IdentityCreateFromAddresses`). `transform_into_action` always produces the
        // optimistic SUCCESS action.

        // Account for the per-key proof-of-possession signature verifications on the SUCCESS path so
        // the metered fee includes their CPU cost — exactly as `IdentityCreate`'s identity-and-
        // signatures stage does. The signatures themselves are verified earlier (in
        // `validate_shielded_proof`, ahead of Halo 2); this records one `SignatureVerification`
        // operation per key WITHOUT re-verifying, so a Type 20 transition is charged for the same
        // signature-verification work as a plain `IdentityCreate`. (Only reached once the bundle
        // proof + PoP have passed, so no nullifier is consumed and no fee charged for a rejected
        // transition.)
        for key in v0.public_keys.iter() {
            execution_context.add_operation(ValidationOperation::SignatureVerification(
                SignatureVerificationOperation::new(key.key_type()),
            ));
        }

        // The action carries the client-predicted fee for reference; the authoritative fee is
        // METERED at execution and moved from the new identity's balance into the fee pools.
        let fee_amount = dpp::shielded::compute_shielded_identity_create_fee(
            v0.actions.len(),
            v0.public_keys.len(),
            platform_version,
        )?;

        let action = IdentityCreateFromShieldedPoolTransitionAction::try_from_transition(
            self,
            current_total_balance,
            fee_amount,
            platform_version,
        )?;

        Ok(ConsensusValidationResult::new_with_data(
            StateTransitionAction::IdentityCreateFromShieldedPoolAction(action),
        ))
    }
}

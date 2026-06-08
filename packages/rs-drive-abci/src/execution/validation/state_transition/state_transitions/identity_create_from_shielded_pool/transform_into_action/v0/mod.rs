use crate::error::Error;
use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::validate_unique_identity_public_key_hashes_in_state::validate_unique_identity_public_key_hashes_not_in_state;
use crate::execution::validation::state_transition::state_transitions::shielded_common::{
    read_pool_total_balance, validate_anchor_exists, validate_minimum_pool_notes,
    validate_nullifiers,
};
use dpp::consensus::state::identity::IdentityAlreadyExistsError;
use dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
use dpp::consensus::state::state_error::StateError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::derive_identity_id_from_actions;
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

        // Identity-creation state checks (mirroring `IdentityCreate`'s `validate_state`), so the
        // `AddNewIdentity` write can't fail as an internal Drive error during execution:
        //
        // 1. The new identity must not already exist. The id is `double_sha256(sorted nullifiers)` —
        //    collision-resistant and derived from single-use spend tags — so this is practically
        //    unreachable, but check explicitly to return a clean consensus rejection. (No asset lock
        //    here, so a failure is a plain rejection, not a partial-asset-lock penalty.)
        let identity_id = derive_identity_id_from_actions(&v0.actions);
        if drive
            .fetch_identity_balance(identity_id.to_buffer(), transaction, platform_version)?
            .is_some()
        {
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityAlreadyExistsError::new(identity_id).into(),
            ));
        }

        // 2. None of the new identity's public-key hashes may already be registered to another
        //    identity (platform enforces globally-unique key hashes for unique key types). Without
        //    this, a duplicate unique key would fail inside the `AddNewIdentity` write at execution
        //    as an internal Drive error instead of a clean consensus rejection.
        let unique_keys_result = validate_unique_identity_public_key_hashes_not_in_state(
            &v0.public_keys,
            drive,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !unique_keys_result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(
                unique_keys_result.errors,
            ));
        }

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

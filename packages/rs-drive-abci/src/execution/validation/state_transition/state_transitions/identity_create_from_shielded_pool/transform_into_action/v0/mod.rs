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
use dpp::serialization::PlatformMessageSignable;
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
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
        signable_bytes: Vec<u8>,
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
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let IdentityCreateFromShieldedPoolTransition::V0(v0) = self;

        let anchor: [u8; 32] = v0.anchor;
        let nullifiers: Vec<[u8; 32]> = v0.actions.iter().map(|a| a.nullifier).collect();

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

        // Validate the new identity's key structure: a master key is required (in_create = true),
        // no duplicate key ids or key data, and each key's security level matches its purpose —
        // identical to `IdentityCreate`.
        let key_structure_result =
            IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
                &v0.public_keys,
                true,
                platform_version,
            )?;
        if !key_structure_result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(
                key_structure_result.errors,
            ));
        }

        // Per-key proof-of-possession: each key must sign the transition's signable bytes, proving
        // the creator controls every key being registered (mirrors `IdentityCreate`'s
        // identity-and-signatures check). The Orchard `extra_sighash_data` binding already pins the
        // exact key set to this spend, so a relayer cannot swap keys; this additionally proves the
        // creator holds them.
        for key in v0.public_keys.iter() {
            let result = signable_bytes.as_slice().verify_signature(
                key.key_type(),
                key.data().as_slice(),
                key.signature().as_slice(),
            );
            execution_context.add_operation(ValidationOperation::SignatureVerification(
                SignatureVerificationOperation::new(key.key_type()),
            ));
            if !result.is_valid() {
                return Ok(ConsensusValidationResult::new_with_errors(result.errors));
            }
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

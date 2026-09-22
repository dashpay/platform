use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::shielded_common::{
    read_pool_total_balance, validate_anchor_exists, validate_minimum_pool_notes,
    validate_nullifiers,
};
use dpp::consensus::signature::IdentityNotFoundError;
use dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
use dpp::consensus::state::state_error::StateError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::identity_top_up_from_shielded_pool::IdentityTopUpFromShieldedPoolTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::identity_top_up_from_shielded_pool) trait IdentityTopUpFromShieldedPoolStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityTopUpFromShieldedPoolStateTransitionTransformIntoActionValidationV0
    for IdentityTopUpFromShieldedPoolTransition
{
    /// The Orchard proof and the flat pool-paid fee floor have already been checked by
    /// the processor. Stateful checks here mirror `Unshield` (pool notes floor, anchor,
    /// unspent nullifiers, pool balance) plus one addition: the credited identity must
    /// already exist. A top-up to an unknown identity is refused rather than creating
    /// an identity with no keys.
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let IdentityTopUpFromShieldedPoolTransition::V0(v0) = self;

        let anchor: [u8; 32] = v0.anchor;
        let nullifiers: Vec<[u8; 32]> = v0.actions.iter().map(|a| a.nullifier).collect();

        let mut drive_operations = vec![];
        let current_total_balance =
            read_pool_total_balance(drive, transaction, &mut drive_operations, platform_version)?;

        if let Some(consensus_error) = validate_minimum_pool_notes(
            drive,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(consensus_error);
        }

        if let Some(consensus_error) = validate_anchor_exists(
            drive,
            &anchor,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(consensus_error);
        }

        if let Some(consensus_error) = validate_nullifiers(
            drive,
            &nullifiers,
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            return Ok(consensus_error);
        }

        if current_total_balance < v0.top_up_amount {
            return Ok(ConsensusValidationResult::new_with_error(
                StateError::InvalidShieldedProofError(InvalidShieldedProofError::new(format!(
                    "shielded pool has insufficient balance: pool has {} but identity top up requires {}",
                    current_total_balance, v0.top_up_amount
                )))
                .into(),
            ));
        }

        if drive
            .fetch_identity_balance(v0.identity_id.to_buffer(), transaction, platform_version)?
            .is_none()
        {
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityNotFoundError::new(v0.identity_id).into(),
            ));
        }

        let fee_amount = dpp::shielded::compute_shielded_identity_top_up_fee(
            v0.actions.len(),
            platform_version,
        )?;

        let result = IdentityTopUpFromShieldedPoolTransitionAction::try_from_transition(
            self,
            current_total_balance,
            fee_amount,
        );

        Ok(result.map(|action| action.into()))
    }
}

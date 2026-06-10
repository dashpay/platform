use crate::error::Error;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::validate_unique_identity_public_key_hashes_in_state::validate_unique_identity_public_key_hashes_not_in_state;
use crate::platform_types::platform::PlatformRef;
use dpp::consensus::state::identity::IdentityAlreadyExistsError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::accessors::IdentityCreateFromShieldedPoolTransitionAccessorsV0;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::derive_identity_id_from_actions;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::identity_create_from_shielded_pool::IdentityCreateFromShieldedPoolTransitionAction;
use drive::state_transition_action::shielded::unshield::v0::UnshieldTransitionActionV0;
use drive::state_transition_action::shielded::unshield::UnshieldTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create_from_shielded_pool) trait IdentityCreateFromShieldedPoolStateTransitionStateValidationV0
{
    fn validate_state_v0<C>(
        &self,
        platform: &PlatformRef<C>,
        action: IdentityCreateFromShieldedPoolTransitionAction,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreateFromShieldedPoolStateTransitionStateValidationV0
    for IdentityCreateFromShieldedPoolTransition
{
    fn validate_state_v0<C>(
        &self,
        platform: &PlatformRef<C>,
        action: IdentityCreateFromShieldedPoolTransitionAction,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let drive = platform.drive;

        // 1. The new identity must not already exist. The id is `double_sha256(sorted nullifiers)` —
        //    collision-resistant and derived from single-use spend tags — so this is practically
        //    unreachable, but check explicitly to return a clean consensus rejection. There is no
        //    chargeable fallback for this case (it cannot be triggered by a relayer choosing a
        //    colliding id), so a failure is a plain free rejection, mirroring the identity-exists
        //    check in `IdentityCreateFromAddresses`'s `validate_state`.
        let identity_id = derive_identity_id_from_actions(self.actions());
        if drive
            .fetch_identity_balance(identity_id.to_buffer(), transaction, platform_version)?
            .is_some()
        {
            // Since the id comes entirely from the spend nullifiers this should never be reachable.
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityAlreadyExistsError::new(identity_id).into(),
            ));
        }

        // 2. None of the new identity's public-key hashes may already be registered to another
        //    identity (platform enforces globally-unique key hashes for unique key types). Unlike the
        //    identity-exists check above, this CAN be triggered by an attacker re-using a victim's
        //    public-key hash, so it gets a chargeable fallback instead of a free rejection: on
        //    failure the spend is still final and the value is credited to
        //    `send_to_address_on_creation_failure` minus a penalty. This is topologically identical
        //    to an `Unshield` (pool -> address minus fee), so we reuse `UnshieldTransitionAction`
        //    wholesale (its converter, `PaidFromShieldedPool` execution event, and conservation).
        let unique_public_key_validation_result =
            validate_unique_identity_public_key_hashes_not_in_state(
                self.public_keys(),
                drive,
                execution_context,
                transaction,
                platform_version,
            )?;

        if unique_public_key_validation_result.is_valid() {
            // We just pass the success action that was built by `transform_into_action`.
            Ok(ConsensusValidationResult::new_with_data(
                StateTransitionAction::IdentityCreateFromShieldedPoolAction(action),
            ))
        } else {
            // A unique-key-hash collision: finalize the spend and credit the fallback address minus a
            // penalty. The penalty is the flat `unique_key_already_present` amount plus the metered
            // processing fee accumulated so far (like `IdentityCreateFromAddresses`'s
            // `BumpAddressInputNonces` penalty) PLUS the flat shielded compute fee
            // (`compute_shielded_verification_fee`): the proposer ran the same Halo 2 verification on
            // the failure path that the success path charges via `additional_fixed_fee_cost`, so the
            // penalty floor must cover it too (fee parity with the success / other shielded paths). We
            // then CAP it at the denomination so the Unshield converter's `amount.checked_sub(fee)`
            // cannot underflow (a net-zero credit is the worst case: the whole spend is consumed by
            // the penalty and flows to the fee pools).
            let denomination = action.denomination();
            let compute_fee = dpp::shielded::compute_shielded_verification_fee(
                action.notes().len(),
                platform_version,
            )?;
            let penalty = platform_version
                .drive_abci
                .validation_and_processing
                .penalties
                .unique_key_already_present
                .checked_add(execution_context.fee_cost(platform_version)?.processing_fee)
                .and_then(|v| v.checked_add(compute_fee))
                .ok_or(ProtocolError::Overflow(
                    "identity create from shielded pool failure penalty overflow",
                ))?
                .min(denomination);

            let failure_action = UnshieldTransitionAction::V0(UnshieldTransitionActionV0 {
                output_address: *self.send_to_address_on_creation_failure(),
                amount: denomination,
                notes: action.notes().to_vec(),
                anchor: *action.anchor(),
                fee_amount: penalty,
                current_total_balance: action.current_total_balance(),
                // This is the chargeable failure of an identity create: the `PaidFromShieldedPool`
                // execution event reads this flag to apply its ops despite the attached collision
                // errors (so the apply-despite-errors path is type-enforced, not comment-enforced).
                chargeable_failure: true,
            });

            Ok(ConsensusValidationResult::new_with_data_and_errors(
                StateTransitionAction::UnshieldAction(failure_action),
                unique_public_key_validation_result.errors,
            ))
        }
    }
}

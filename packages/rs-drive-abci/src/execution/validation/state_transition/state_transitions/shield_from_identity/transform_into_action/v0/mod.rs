use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::shielded_common::read_pool_total_balance;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::shield_from_identity::ShieldFromIdentityTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::shield_from_identity) trait ShieldFromIdentityStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ShieldFromIdentityStateTransitionTransformIntoActionValidationV0
    for ShieldFromIdentityTransition
{
    /// The identity signature, nonce, balance floor, structure, and Orchard proof
    /// have all been checked by the processor before this point. The identity is
    /// debited exactly `amount`, so unlike `Shield` there is no per-input slack to
    /// reallocate; only the pool total is read so the action can bump it.
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut drive_operations = vec![];
        let current_total_balance =
            read_pool_total_balance(drive, transaction, &mut drive_operations, platform_version)?;

        let result =
            ShieldFromIdentityTransitionAction::try_from_transition(self, current_total_balance);

        Ok(result.map(|action| action.into()))
    }
}

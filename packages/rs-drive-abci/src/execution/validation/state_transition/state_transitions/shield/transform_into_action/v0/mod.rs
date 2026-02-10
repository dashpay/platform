use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::shield_transition::ShieldTransition;
use dpp::version::PlatformVersion;
use drive::state_transition_action::shielded::shield::ShieldTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeMap;

pub(in crate::execution::validation::state_transition::state_transitions::shield) trait ShieldStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ShieldStateTransitionTransformIntoActionValidationV0 for ShieldTransition {
    fn transform_into_action_v0(
        &self,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        _platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        // Extract note commitments and encrypted notes from serialized actions
        let note_commitments: Vec<[u8; 32]> = match self {
            ShieldTransition::V0(v0) => v0.actions.iter().map(|a| a.cmx).collect(),
        };
        let encrypted_notes: Vec<Vec<u8>> = match self {
            ShieldTransition::V0(v0) => {
                v0.actions.iter().map(|a| a.encrypted_note.clone()).collect()
            }
        };

        // The value_balance is negative for shield (funds flowing into the pool)
        let shield_amount = match self {
            ShieldTransition::V0(v0) => (-v0.value_balance) as u64,
        };

        // TODO: Read current shielded pool state from GroveDB.
        // These should be fetched from the shielded pool tree in GroveDB:
        //   - current_checkpoint_id: the latest checkpoint epoch ID
        //   - current_total_balance: the running total balance of the shielded pool
        // For now, use default values until the GroveDB API for shielded pool is available.
        let current_checkpoint_id: u64 = 0;
        let current_total_balance: Credits = 0;

        let result = ShieldTransitionAction::try_from_transition(
            self,
            inputs_with_remaining_balance,
            shield_amount,
            note_commitments,
            encrypted_notes,
            current_checkpoint_id,
            current_total_balance,
        );

        Ok(result.map(|action| action.into()))
    }
}

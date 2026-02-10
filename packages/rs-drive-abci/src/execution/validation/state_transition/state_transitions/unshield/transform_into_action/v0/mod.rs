use crate::error::Error;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::unshield_transition::UnshieldTransition;
use dpp::version::PlatformVersion;
use drive::state_transition_action::shielded::unshield::UnshieldTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::unshield) trait UnshieldStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl UnshieldStateTransitionTransformIntoActionValidationV0 for UnshieldTransition {
    fn transform_into_action_v0(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        // Extract nullifiers, note commitments, and encrypted notes from serialized actions
        let nullifiers: Vec<[u8; 32]> = match self {
            UnshieldTransition::V0(v0) => v0.actions.iter().map(|a| a.nullifier).collect(),
        };
        let note_commitments: Vec<[u8; 32]> = match self {
            UnshieldTransition::V0(v0) => v0.actions.iter().map(|a| a.cmx).collect(),
        };
        let encrypted_notes: Vec<Vec<u8>> = match self {
            UnshieldTransition::V0(v0) => {
                v0.actions.iter().map(|a| a.encrypted_note.clone()).collect()
            }
        };

        // The anchor from the transition (Merkle root of commitment tree)
        let anchor: [u8; 32] = match self {
            UnshieldTransition::V0(v0) => v0.anchor,
        };

        // TODO: Read current shielded pool state from GroveDB.
        // These should be fetched from the shielded pool tree in GroveDB:
        //   - current_checkpoint_id: the latest checkpoint epoch ID
        //   - current_total_balance: the running total balance of the shielded pool
        // For now, use default values until the GroveDB API for shielded pool is available.
        let current_checkpoint_id: u64 = 0;
        let current_total_balance: Credits = 0;

        // TODO: Verify the anchor exists in the commitment tree before proceeding.

        let result = UnshieldTransitionAction::try_from_transition(
            self,
            nullifiers,
            note_commitments,
            encrypted_notes,
            anchor,
            current_checkpoint_id,
            current_total_balance,
        );

        Ok(result.map(|action| action.into()))
    }
}

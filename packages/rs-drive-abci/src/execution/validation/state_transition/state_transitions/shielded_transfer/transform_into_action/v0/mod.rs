use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::shielded_common::reconstruct_and_verify_bundle;
use dpp::consensus::state::shielded::invalid_anchor_error::InvalidAnchorError;
use dpp::consensus::state::shielded::nullifier_already_spent_error::NullifierAlreadySpentError;
use dpp::consensus::state::state_error::StateError;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::shielded::ShieldedPoolParams;
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{
    shielded_anchors_credit_pool_path, shielded_credit_pool_nullifiers_path,
    shielded_credit_pool_path, SHIELDED_PARAMS_KEY, SHIELDED_TOTAL_BALANCE_KEY,
};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::shielded_transfer::ShieldedTransferTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use drive::util::grove_operations::DirectQueryType;

pub(in crate::execution::validation::state_transition::state_transitions::shielded_transfer) trait ShieldedTransferStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ShieldedTransferStateTransitionTransformIntoActionValidationV0
    for ShieldedTransferTransition
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        // Extract nullifiers, note commitments, and encrypted notes from serialized actions
        let nullifiers: Vec<[u8; 32]> = match self {
            ShieldedTransferTransition::V0(v0) => {
                v0.actions.iter().map(|a| a.nullifier).collect()
            }
        };
        let note_commitments: Vec<[u8; 32]> = match self {
            ShieldedTransferTransition::V0(v0) => v0.actions.iter().map(|a| a.cmx).collect(),
        };
        let encrypted_notes: Vec<Vec<u8>> = match self {
            ShieldedTransferTransition::V0(v0) => {
                v0.actions.iter().map(|a| a.encrypted_note.clone()).collect()
            }
        };

        // The anchor from the transition (Merkle root of commitment tree)
        let anchor: [u8; 32] = match self {
            ShieldedTransferTransition::V0(v0) => v0.anchor,
        };

        // The value_balance is positive for shielded transfer (fee extracted from pool)
        let fee_amount: Credits = match self {
            ShieldedTransferTransition::V0(v0) => v0.value_balance as u64,
        };

        // Read current shielded pool state from GroveDB
        let mut drive_operations = vec![];
        let pool_path = shielded_credit_pool_path();

        let params_bytes = drive.grove_get_raw_item(
            (&pool_path).into(),
            &[SHIELDED_PARAMS_KEY],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;
        let (params, _): (ShieldedPoolParams, _) =
            bincode::decode_from_slice(&params_bytes, bincode::config::standard())
                .map_err(|e| {
                    Error::Protocol(
                        dpp::ProtocolError::DecodingError(format!(
                            "could not decode shielded pool params: {e}"
                        )),
                    )
                })?;
        let current_checkpoint_id = params.checkpoint_id_counter;

        let current_total_balance = drive
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&pool_path).into(),
                &[SHIELDED_TOTAL_BALANCE_KEY],
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?
            .unwrap_or(0);

        // Verify the anchor exists in the recorded anchors tree
        let anchors_path = shielded_anchors_credit_pool_path();
        let anchor_exists = drive.grove_has_raw(
            (&anchors_path).into(),
            &anchor,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        if !anchor_exists {
            return Ok(ConsensusValidationResult::new_with_error(
                StateError::InvalidAnchorError(InvalidAnchorError::new(anchor)).into(),
            ));
        }

        // Check that no nullifier has already been spent
        let nullifiers_path = shielded_credit_pool_nullifiers_path();
        for nullifier in &nullifiers {
            let exists = drive.grove_has_raw(
                (&nullifiers_path).into(),
                nullifier,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;

            if exists {
                return Ok(ConsensusValidationResult::new_with_error(
                    StateError::NullifierAlreadySpentError(
                        NullifierAlreadySpentError::new(*nullifier),
                    )
                    .into(),
                ));
            }
        }

        // Verify the ZK proof
        let (st_actions, st_flags, st_value_balance, st_proof, st_binding_sig) = match self {
            ShieldedTransferTransition::V0(v0) => (
                &v0.actions,
                v0.flags,
                v0.value_balance,
                v0.proof.as_slice(),
                v0.binding_signature.as_slice(),
            ),
        };

        if let Err(e) = reconstruct_and_verify_bundle(
            st_actions,
            st_flags,
            st_value_balance,
            &anchor,
            st_proof,
            st_binding_sig,
        ) {
            return Ok(ConsensusValidationResult::new_with_error(
                StateError::InvalidShieldedProofError(e).into(),
            ));
        }

        let result = ShieldedTransferTransitionAction::try_from_transition(
            self,
            nullifiers,
            note_commitments,
            encrypted_notes,
            anchor,
            fee_amount,
            current_checkpoint_id,
            current_total_balance,
        );

        Ok(result.map(|action| action.into()))
    }
}

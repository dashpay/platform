use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::shielded_common::reconstruct_and_verify_bundle;
use dpp::address_funds::PlatformAddress;
use dpp::consensus::state::state_error::StateError;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::shielded::ShieldedPoolParams;
use dpp::state_transition::shield_transition::ShieldTransition;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{
    shielded_credit_pool_path, SHIELDED_PARAMS_KEY, SHIELDED_TOTAL_BALANCE_KEY,
};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::shield::ShieldTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use drive::util::grove_operations::DirectQueryType;
use std::collections::BTreeMap;

pub(in crate::execution::validation::state_transition::state_transitions::shield) trait ShieldStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ShieldStateTransitionTransformIntoActionValidationV0 for ShieldTransition {
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        platform_version: &PlatformVersion,
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

        // Verify the ZK proof
        let (actions, flags, value_balance, anchor, proof, binding_signature) = match self {
            ShieldTransition::V0(v0) => (
                &v0.actions,
                v0.flags,
                v0.value_balance,
                &v0.anchor,
                v0.proof.as_slice(),
                v0.binding_signature.as_slice(),
            ),
        };

        if let Err(e) = reconstruct_and_verify_bundle(
            actions,
            flags,
            value_balance,
            anchor,
            proof,
            binding_signature,
        ) {
            return Ok(ConsensusValidationResult::new_with_error(
                StateError::InvalidShieldedProofError(e).into(),
            ));
        }

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

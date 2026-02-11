use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::shielded_common::reconstruct_and_verify_bundle;
use dpp::address_funds::PlatformAddress;
use dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
use dpp::consensus::state::state_error::StateError;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::shield_transition::ShieldTransition;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{shielded_credit_pool_path, SHIELDED_TOTAL_BALANCE_KEY};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::shield::ShieldTransitionAction;
use drive::state_transition_action::system::bump_address_input_nonces_action::BumpAddressInputNoncesAction;
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
            ShieldTransition::V0(v0) => v0
                .actions
                .iter()
                .map(|a| a.encrypted_note.clone())
                .collect(),
        };

        // The value_balance must be negative for shield (funds flowing into the pool).
        // Safely negate to get the shield amount, guarding against overflow (i64::MIN).
        let shield_amount: Credits = match self {
            ShieldTransition::V0(v0) => {
                if v0.value_balance >= 0 {
                    return Ok(ConsensusValidationResult::new_with_error(
                        StateError::InvalidShieldedProofError(InvalidShieldedProofError::new(
                            "shield value_balance must be negative".to_string(),
                        ))
                        .into(),
                    ));
                }
                match v0.value_balance.checked_neg() {
                    Some(positive) => positive as u64,
                    None => {
                        return Ok(ConsensusValidationResult::new_with_error(
                            StateError::InvalidShieldedProofError(InvalidShieldedProofError::new(
                                "shield value_balance overflow (i64::MIN)".to_string(),
                            ))
                            .into(),
                        ));
                    }
                }
            }
        };

        // Read current shielded pool state from GroveDB
        let mut drive_operations = vec![];
        let pool_path = shielded_credit_pool_path();

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
                &v0.binding_signature,
            ),
        };

        if let Err(e) = reconstruct_and_verify_bundle(
            actions,
            flags,
            value_balance,
            anchor,
            proof,
            binding_signature,
            &[], // No transparent fields to bind for shield
        ) {
            let penalty = platform_version
                .drive_abci
                .validation_and_processing
                .penalties
                .shielded_proof_verification_failure;

            let (fee_strategy, user_fee_increase) = match self {
                ShieldTransition::V0(v0) => (&v0.fee_strategy, v0.user_fee_increase),
            };

            let bump_action = StateTransitionAction::BumpAddressInputNoncesAction(
                BumpAddressInputNoncesAction::from_inputs_with_remaining_balance(
                    &inputs_with_remaining_balance,
                    fee_strategy,
                    penalty,
                    user_fee_increase,
                ),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![StateError::InvalidShieldedProofError(e).into()],
            ));
        }

        let result = ShieldTransitionAction::try_from_transition(
            self,
            inputs_with_remaining_balance,
            shield_amount,
            note_commitments,
            encrypted_notes,
            current_total_balance,
        );

        Ok(result.map(|action| action.into()))
    }
}

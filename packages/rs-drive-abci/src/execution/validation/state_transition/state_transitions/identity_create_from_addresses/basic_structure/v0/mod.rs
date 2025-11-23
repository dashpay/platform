use dpp::consensus::basic::BasicError;
use crate::error::Error;
use dpp::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;
use dpp::consensus::state::state_error::StateError;
use dpp::identity::state_transition::AssetLockProved;
use dpp::prelude::ConsensusValidationResult;
use dpp::ProtocolError;
use dpp::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;
use drive::state_transition_action::system::partially_use_asset_lock_action::PartiallyUseAssetLockAction;
use crate::execution::validation::state_transition::identity_create_from_addresses::identity_and_signatures::v0::IdentityCreateFromAddressesStateTransitionIdentityAndSignaturesValidationV0;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create_from_addresses) trait IdentityCreateFromAddressesStateTransitionBasicStructureValidationV0
{
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreateFromAddressesStateTransitionBasicStructureValidationV0
    for IdentityCreateFromAddressesTransition
{
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        if self.inputs().len() > platform_version.dpp.state_transitions.max_inputs as usize {
            Ok(SimpleConsensusValidationResult::new_with_error(
                BasicError::TransitionOverMaxInputsError(TransitionOverMaxInputsError::new(
                    platform_version.dpp.state_transitions.max_inputs as usize,
                ))
                .into(),
            ))
        } else {
            Ok(SimpleConsensusValidationResult::new())
        }

        if self.inputs().len() != self.input_witnesses().len() {}

        if self.public_keys().len()
            > platform_version
                .dpp
                .state_transitions
                .identities
                .max_public_keys_in_creation as usize
        {
            Ok(SimpleConsensusValidationResult::new_with_error(
                StateError::MaxIdentityPublicKeyLimitReachedError(
                    MaxIdentityPublicKeyLimitReachedError::new(
                        platform_version
                            .dpp
                            .state_transitions
                            .identities
                            .max_public_keys_in_creation as usize,
                    ),
                )
                .into(),
            ))
        } else {
            Ok(SimpleConsensusValidationResult::new())
        }

        let validation_result =
            IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
                self.public_keys(),
                true,
                platform_version,
            )
            .map_err(Error::Protocol)?;

        if !validation_result.is_valid() {
            let penalty = platform_version
                .drive_abci
                .validation_and_processing
                .penalties
                .validation_of_added_keys_structure_failure;

            let used_credits = penalty
                .checked_add(execution_context.fee_cost(platform_version)?.processing_fee)
                .ok_or(ProtocolError::Overflow("processing fee overflow error"))?;

            let bump_action = StateTransitionAction::PartiallyUseAssetLockAction(
                PartiallyUseAssetLockAction::from_borrowed_identity_create_from_addresses_transition_action(
                    action,
                    used_credits,
                ),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                validation_result.errors,
            ));
        }

        // Now we should validate proof of possession
        let validation_result = self
            .validate_identity_create_from_addresses_state_transition_signatures_v0(
                signable_bytes,
                execution_context,
            );

        if !validation_result.is_valid() {
            let penalty = platform_version
                .drive_abci
                .validation_and_processing
                .penalties
                .validation_of_added_keys_proof_of_possession_failure;

            let used_credits = penalty
                .checked_add(execution_context.fee_cost(platform_version)?.processing_fee)
                .ok_or(ProtocolError::Overflow("processing fee overflow error"))?;

            let bump_action = StateTransitionAction::PartiallyUseAssetLockAction(
                PartiallyUseAssetLockAction::from_borrowed_identity_create_from_addresses_transition_action(
                    action,
                    used_credits,
                ),
            );

            Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                validation_result.errors,
            ))
        } else {
            Ok(ConsensusValidationResult::new())
        }
    }
}

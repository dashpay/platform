use dpp::consensus::basic::BasicError;
use dpp::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use dpp::consensus::ConsensusError;
use dpp::identity::state_transition::AssetLockProved;
use crate::error::Error;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use drive::state_transition_action::system::partially_use_asset_lock_action::PartiallyUseAssetLockAction;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create) trait IdentityCreateStateTransitionAdvancedStructureValidationV0
{
    fn validate_advanced_structure_from_state_v0(
        &self,
        // The state here is only to be used to query information for the action
        action: &IdentityCreateTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreateStateTransitionAdvancedStructureValidationV0 for IdentityCreateTransition {
    fn validate_advanced_structure_from_state_v0(
        &self,
        action: &IdentityCreateTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {

        // We should validate that the identity id is created from the asset lock proof

        // We don't need to return a consensus error here because the outpoint will already have been checked in the transformation into an action
        let identifier_from_outpoint = self.asset_lock_proof().create_identifier()?;

        if identifier_from_outpoint != self.identity_id() {

            let penalty = platform_version.drive_abci.validation_and_processing.penalties.identity_id_not_correct;

            // todo: pay for processing
            let used_credits = penalty; // + processing

            // Most probably an attempted attack
            let bump_action = StateTransitionAction::PartiallyUseAssetLockAction(
                PartiallyUseAssetLockAction::from_borrowed_identity_create_transition_action(action, used_credits)
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![ConsensusError::BasicError(
                    BasicError::InvalidIdentifierError(InvalidIdentifierError::new(
                        "identity_id".to_string(),
                        "does not match created identifier from asset lock".to_string(),
                    )),
                )],
            ));
        }
        
        let validation_result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            self.public_keys(),
            true,
            platform_version,
        )
        .map_err(Error::Protocol)?;
        

        if !validation_result.is_valid() {

            let penalty = platform_version.drive_abci.validation_and_processing.penalties.validation_of_added_keys_failure;

            // todo: pay for processing
            let used_credits = penalty; // + processing
            
            let bump_action = StateTransitionAction::PartiallyUseAssetLockAction(
                PartiallyUseAssetLockAction::from_borrowed_identity_create_transition_action(action, used_credits)
            );

            Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                validation_result.errors,
                ),
            )
        } else {
            Ok(ConsensusValidationResult::new())
        }
    }
}

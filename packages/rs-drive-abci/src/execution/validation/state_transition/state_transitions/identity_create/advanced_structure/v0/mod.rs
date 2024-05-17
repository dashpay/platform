use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::identity_create::identity_and_signatures::v0::IdentityCreateStateTransitionIdentityAndSignaturesValidationV0;
use dpp::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::identity::state_transition::AssetLockProved;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;
use drive::state_transition_action::system::partially_use_asset_lock_action::PartiallyUseAssetLockAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create) trait IdentityCreateStateTransitionAdvancedStructureValidationV0
{
    fn validate_advanced_structure_from_state_v0(
        &self,
        action: &IdentityCreateTransitionAction,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreateStateTransitionAdvancedStructureValidationV0 for IdentityCreateTransition {
    fn validate_advanced_structure_from_state_v0(
        &self,
        action: &IdentityCreateTransitionAction,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        // We should validate that the identity id is created from the asset lock proof

        // We don't need to return a consensus error here because the outpoint will already have been checked in the transformation into an action
        let identifier_from_outpoint = self.asset_lock_proof().create_identifier()?;

        // Creating an identifier costs 1 block hash (64 bytes)
        // The cost should come here after, as the protocol error is just if no outpoint exists

        execution_context.add_operation(ValidationOperation::DoubleSha256(1));

        if identifier_from_outpoint != self.identity_id() {
            let penalty = platform_version
                .drive_abci
                .validation_and_processing
                .penalties
                .identity_id_not_correct;

            let used_credits = penalty
                .checked_add(execution_context.fee_cost(platform_version)?.processing_fee)
                .ok_or(ProtocolError::Overflow("processing fee overflow error"))?;

            // Most probably an attempted attack
            let bump_action = StateTransitionAction::PartiallyUseAssetLockAction(
                PartiallyUseAssetLockAction::from_borrowed_identity_create_transition_action(
                    action,
                    used_credits,
                ),
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
                PartiallyUseAssetLockAction::from_borrowed_identity_create_transition_action(
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
        let validation_result = self.validate_identity_create_state_transition_signatures_v0(
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
                PartiallyUseAssetLockAction::from_borrowed_identity_create_transition_action(
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

use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::identity_create::identity_and_signatures::v0::IdentityCreateStateTransitionIdentityAndSignaturesValidationV0;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;
use drive::state_transition_action::system::partially_use_asset_lock_action::PartiallyUseAssetLockAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create) trait IdentityCreateStateTransitionAdvancedStructureValidationV2
{
    fn validate_advanced_structure_from_state_v2(
        &self,
        action: &IdentityCreateTransitionAction,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreateStateTransitionAdvancedStructureValidationV2 for IdentityCreateTransition {
    /// Like v1, except that the identity id is not checked here: basic structure validation 1
    /// checks it before the asset lock proof is verified.
    ///
    /// A key whose proof of possession fails is refused unpaid, as in v1: the asset lock
    /// signature covers the transition's signable bytes, which leave out the key signatures. A
    /// key structure failure is still charged, because the keys are part of the signed bytes.
    fn validate_advanced_structure_from_state_v2(
        &self,
        action: &IdentityCreateTransitionAction,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        // Basic structure, which has no execution context, derived the identifier from the
        // asset lock's outpoint. Its double SHA-256 is charged here, where v1 derived it, so an
        // identity create costs what it did under v1.
        execution_context.add_operation(ValidationOperation::DoubleSha256(1));

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

        let validation_result = self.validate_identity_create_state_transition_signatures_v0(
            signable_bytes,
            execution_context,
        );

        if validation_result.is_valid() {
            Ok(ConsensusValidationResult::new())
        } else {
            // No action: the failure is refused unpaid
            Ok(ConsensusValidationResult::new_with_errors(
                validation_result.errors,
            ))
        }
    }
}

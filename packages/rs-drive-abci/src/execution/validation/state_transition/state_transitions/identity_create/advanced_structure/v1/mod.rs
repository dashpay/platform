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

pub(in crate::execution::validation::state_transition::state_transitions::identity_create) trait IdentityCreateStateTransitionAdvancedStructureValidationV1
{
    fn validate_advanced_structure_from_state_v1(
        &self,
        action: &IdentityCreateTransitionAction,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreateStateTransitionAdvancedStructureValidationV1 for IdentityCreateTransition {
    /// Like v0, except that an identity id other than the one the asset lock derives and a key
    /// whose proof of possession fails are refused unpaid.
    ///
    /// The asset lock signature covers the transition's signable bytes, which leave out the
    /// identity id and the key signatures, so an identity id that is not the derived one, or a
    /// key signature that does not verify, is in a field the asset lock signature does not
    /// cover. Such a failure is not charged to the asset lock: the transition is refused like
    /// one whose asset lock signature fails, and the asset lock is left as it was.
    ///
    /// The data of a key that carries a proof of possession is signed, yet a key whose data
    /// does not parse fails that proof too, and is refused unpaid like any other proof failure.
    /// Only the holder of the asset lock key can sign such data, and check_tx refuses it. A key
    /// structure failure is still charged, because the keys are part of the signed bytes.
    fn validate_advanced_structure_from_state_v1(
        &self,
        action: &IdentityCreateTransitionAction,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        // The outpoint was already checked when the transition was transformed into an action,
        // so failing to derive the identifier here is not a consensus error
        let identifier_from_outpoint = self.asset_lock_proof().create_identifier()?;

        // Deriving the identifier costs one double SHA-256
        execution_context.add_operation(ValidationOperation::DoubleSha256(1));

        if identifier_from_outpoint != self.identity_id() {
            // No action: the failure is refused unpaid
            return Ok(ConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::InvalidIdentifierError(
                    InvalidIdentifierError::new(
                        "identity_id".to_string(),
                        "does not match created identifier from asset lock".to_string(),
                    ),
                )),
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

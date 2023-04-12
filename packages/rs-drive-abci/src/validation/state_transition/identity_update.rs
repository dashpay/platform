use dpp::identity::PartialIdentity;
use dpp::{identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition, state_transition::StateTransitionAction, StateError, validation::{ConsensusValidationResult, SimpleConsensusValidationResult}};
use dpp::block_time_window::validate_time_in_block_time_window::validate_time_in_block_time_window;
use dpp::identity::state_transition::identity_update_transition::IdentityUpdateTransitionAction;
use dpp::identity::state_transition::identity_update_transition::validate_identity_update_transition_basic::{IDENTITY_UPDATE_JSON_SCHEMA_VALIDATOR, IDENTITY_UPDATE_SCHEMA};
use dpp::validation::ValidationResult;
use drive::grovedb::TransactionArg;
use drive::{drive::Drive, grovedb::Transaction};

use crate::error::execution::ExecutionError;
use crate::error::execution::ExecutionError::CorruptedCodeExecution;
use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::common::{validate_protocol_version, validate_schema};
use crate::validation::state_transition::key_validation::{
    validate_identity_public_key_ids_exist_in_state, validate_identity_public_keys_signatures,
    validate_identity_public_keys_structure, validate_state_transition_identity_signature,
    validate_unique_identity_public_key_hashes_state,
};

use super::StateTransitionValidation;

impl StateTransitionValidation for IdentityUpdateTransition {
    fn validate_structure(
        &self,
        _drive: &Drive,
        _tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema(&IDENTITY_UPDATE_JSON_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_protocol_version(self.protocol_version);
        if !result.is_valid() {
            return Ok(result);
        }

        validate_identity_public_keys_structure(self.add_public_keys.as_slice())
    }

    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        let mut result = ConsensusValidationResult::<Option<PartialIdentity>>::default();
        result.add_errors(
            validate_identity_public_keys_signatures(self.add_public_keys.as_slice())?.errors,
        );

        if !result.is_valid() {
            return Ok(result);
        }

        let validation_result =
            validate_state_transition_identity_signature(drive, self, false, transaction)?;

        if !result.is_valid() {
            result.merge(validation_result);
            return Ok(result);
        }

        let partial_identity = validation_result.into_data()?;

        let Some(revision) = partial_identity.revision else {
            return Err(Error::Execution(CorruptedCodeExecution("revision should exist")));
        };

        // Check revision
        if revision != (self.revision - 1) {
            result.add_error(StateError::InvalidIdentityRevisionError {
                identity_id: self.identity_id,
                current_revision: revision,
            });
            return Ok(result);
        }

        result.set_data(Some(partial_identity));

        Ok(result)
    }

    fn validate_state<'a, C: CoreRPCLike>(
        &self,
        platform: &'a PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let drive = platform.drive;
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::default();

        // Now we should check the state of added keys to make sure there aren't any that already exist
        validation_result.add_errors(
            validate_unique_identity_public_key_hashes_state(
                self.add_public_keys.as_slice(),
                drive,
                tx,
            )?
            .errors,
        );

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        if !self.disable_public_keys.is_empty() {
            // We need to validate that all keys removed existed
            validation_result.add_errors(
                validate_identity_public_key_ids_exist_in_state(
                    self.identity_id,
                    self.disable_public_keys.clone(),
                    drive,
                    tx,
                )?
                .errors,
            );

            if !validation_result.is_valid() {
                return Ok(validation_result);
            }

            validation_result.add_errors(
                validate_identity_public_key_ids_exist_in_state(
                    self.identity_id,
                    self.disable_public_keys.clone(),
                    drive,
                    tx,
                )?
                .errors,
            );

            if !validation_result.is_valid() {
                return Ok(validation_result);
            }

            if let Some(disabled_at_ms) = self.public_keys_disabled_at {
                // We need to verify the time the keys were disabled

                let last_block_time = platform.state.last_block_time_ms().ok_or(
                    Error::Execution(ExecutionError::StateNotInitialized(
                        "expected a last platform block during identity update validation",
                    )),
                )?;

                let window_validation_result = validate_time_in_block_time_window(
                    last_block_time,
                    disabled_at_ms,
                    platform.config.block_spacing_ms,
                );

                if !window_validation_result.is_valid() {
                    validation_result.add_error(
                        StateError::IdentityPublicKeyDisabledAtWindowViolationError {
                            disabled_at: disabled_at_ms,
                            time_window_start: window_validation_result.time_window_start,
                            time_window_end: window_validation_result.time_window_end,
                        },
                    );
                    return Ok(validation_result);
                }
            }
        }

        validation_result.set_data(IdentityUpdateTransitionAction::from(self).into());
        return Ok(validation_result);
    }
}

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use dpp::consensus::state::identity::identity_public_key_disabled_at_window_violation_error::IdentityPublicKeyDisabledAtWindowViolationError;
use dpp::consensus::state::state_error::StateError;

use dpp::prelude::ConsensusValidationResult;

use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::state_transition_action::identity::identity_update::IdentityUpdateTransitionAction;
use dpp::state_transition_action::StateTransitionAction;
use dpp::validation::block_time_window::validate_time_in_block_time_window::validate_time_in_block_time_window;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

use drive::grovedb::TransactionArg;
use crate::execution::validation::state_transition::common::validate_identity_public_key_ids_dont_exist_in_state::v0::validate_identity_public_key_ids_dont_exist_in_state_v0;
use crate::execution::validation::state_transition::common::validate_identity_public_key_ids_exist_in_state::v0::validate_identity_public_key_ids_exist_in_state_v0;
use crate::execution::validation::state_transition::common::validate_unique_identity_public_key_hashes_in_state::v0::validate_unique_identity_public_key_hashes_in_state_v0;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

pub(crate) trait StateTransitionStateValidationV0 {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0(
        &self,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionStateValidationV0 for IdentityUpdateTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let drive = platform.drive;
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::default();

        // Now we should check the state of added keys to make sure there aren't any that already exist
        validation_result.add_errors(
            validate_unique_identity_public_key_hashes_in_state_v0(
                self.add_public_keys.as_slice(),
                drive,
                tx,
                platform_version,
            )?
            .errors,
        );

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        validation_result.add_errors(
            validate_identity_public_key_ids_dont_exist_in_state_v0(
                self.identity_id,
                self.add_public_keys.as_slice(),
                drive,
                tx,
                platform_version,
            )?
            .errors,
        );

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        if !self.disable_public_keys.is_empty() {
            // We need to validate that all keys removed existed
            validation_result.add_errors(
                validate_identity_public_key_ids_exist_in_state_v0(
                    self.identity_id,
                    self.disable_public_keys.clone(),
                    drive,
                    tx,
                    platform_version,
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
                    platform_version,
                )
                .map_err(|e| Error::Protocol(ProtocolError::NonConsensusError(e)))?;

                if !window_validation_result.is_valid() {
                    validation_result.add_error(
                        StateError::IdentityPublicKeyDisabledAtWindowViolationError(
                            IdentityPublicKeyDisabledAtWindowViolationError::new(
                                disabled_at_ms,
                                window_validation_result.time_window_start,
                                window_validation_result.time_window_end,
                            ),
                        ),
                    );
                    return Ok(validation_result);
                }
            }
        }
        self.transform_into_action_v0()
    }

    fn transform_into_action_v0(
        &self,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::default();

        validation_result.set_data(IdentityUpdateTransitionAction::from(self).into());
        Ok(validation_result)
    }
}

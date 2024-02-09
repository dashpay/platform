use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use dpp::consensus::state::identity::identity_public_key_disabled_at_window_violation_error::IdentityPublicKeyDisabledAtWindowViolationError;
use dpp::consensus::state::state_error::StateError;

use dpp::prelude::ConsensusValidationResult;

use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::validation::block_time_window::validate_time_in_block_time_window::validate_time_in_block_time_window;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::state_transition_action::identity::identity_update::IdentityUpdateTransitionAction;
use drive::state_transition_action::StateTransitionAction;

use drive::grovedb::TransactionArg;
use dpp::version::DefaultForPlatformVersion;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_identity_public_key_contract_bounds::validate_identity_public_keys_contract_bounds;
use crate::execution::validation::state_transition::common::validate_identity_public_key_ids_dont_exist_in_state::validate_identity_public_key_ids_dont_exist_in_state;
use crate::execution::validation::state_transition::common::validate_identity_public_key_ids_exist_in_state::validate_identity_public_key_ids_exist_in_state;
use crate::execution::validation::state_transition::common::validate_unique_identity_public_key_hashes_in_state::validate_unique_identity_public_key_hashes_in_state;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

pub(in crate::execution::validation::state_transition::state_transitions::identity_update) trait IdentityUpdateStateTransitionStateValidationV0
{
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

impl IdentityUpdateStateTransitionStateValidationV0 for IdentityUpdateTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut state_transition_execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)?;
        let drive = platform.drive;
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::default();

        // Now we should check the state of added keys to make sure there aren't any that already exist
        validation_result.add_errors(
            validate_unique_identity_public_key_hashes_in_state(
                self.public_keys_to_add(),
                drive,
                &mut state_transition_execution_context,
                tx,
                platform_version,
            )?
            .errors,
        );

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        validation_result.add_errors(
            validate_identity_public_key_ids_dont_exist_in_state(
                self.identity_id(),
                self.public_keys_to_add(),
                drive,
                tx,
                &mut state_transition_execution_context,
                platform_version,
            )?
            .errors,
        );

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // Now we should check to make sure any keys that are added are valid for the contract
        // bounds they refer to
        validation_result.add_errors(
            validate_identity_public_keys_contract_bounds(
                self.identity_id(),
                self.public_keys_to_add(),
                drive,
                tx,
                &mut state_transition_execution_context,
                platform_version,
            )?
            .errors,
        );

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        if !self.public_key_ids_to_disable().is_empty() {
            // We need to validate that all keys removed existed
            validation_result.add_errors(
                validate_identity_public_key_ids_exist_in_state(
                    self.identity_id(),
                    self.public_key_ids_to_disable(),
                    drive,
                    &mut state_transition_execution_context,
                    tx,
                    platform_version,
                )?
                .errors,
            );

            if !validation_result.is_valid() {
                return Ok(validation_result);
            }

            if let Some(disabled_at_ms) = self.public_keys_disabled_at() {
                // We need to verify the time the keys were disabled

                let last_block_time =
                    platform
                        .state
                        .last_committed_block_time_ms()
                        .ok_or(Error::Execution(ExecutionError::StateNotInitialized(
                            "expected a last platform block during identity update validation",
                        )))?;

                let window_validation_result = validate_time_in_block_time_window(
                    last_block_time,
                    disabled_at_ms,
                    platform.config.block_spacing_ms,
                    platform_version,
                )
                .map_err(|e| Error::Protocol(ProtocolError::NonConsensusError(e)))?;

                if !window_validation_result.valid {
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

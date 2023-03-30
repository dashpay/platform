use anyhow::anyhow;
use platform_value::Value;
use std::convert::TryInto;
use std::sync::Arc;

use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::signature::IdentityNotFoundError;
use crate::consensus::state::state_error::StateError;
use crate::identity::state_transition::identity_update_transition::IdentityUpdateTransitionAction;
use crate::{
    block_time_window::validate_time_in_block_time_window::validate_time_in_block_time_window,
    identity::validation::{RequiredPurposeAndSecurityLevelValidator, TPublicKeysValidator},
    state_repository::StateRepositoryLike,
    state_transition::StateTransitionLike,
    validation::ValidationResult,
    NonConsensusError,
};

use super::identity_update_transition::{property_names, IdentityUpdateTransition};

pub struct IdentityUpdateTransitionStateValidator<T, ST> {
    state_repository: Arc<ST>,
    public_keys_validator: Arc<T>,
}

impl<T, SR> IdentityUpdateTransitionStateValidator<T, SR>
where
    T: TPublicKeysValidator,
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: Arc<SR>, public_keys_validator: Arc<T>) -> Self {
        IdentityUpdateTransitionStateValidator {
            state_repository,
            public_keys_validator,
        }
    }

    pub async fn validate(
        &self,
        state_transition: &IdentityUpdateTransition,
    ) -> Result<ValidationResult<IdentityUpdateTransitionAction>, NonConsensusError> {
        let mut validation_result = ValidationResult::default();

        let maybe_stored_identity = self
            .state_repository
            .fetch_identity(
                state_transition.get_identity_id(),
                Some(state_transition.get_execution_context()),
            )
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)
            .map_err(|e| {
                NonConsensusError::StateRepositoryFetchError(format!(
                    "state repository fetch identity for identity update validation error: {}",
                    e.to_string()
                ))
            })?;

        if state_transition.get_execution_context().is_dry_run() {
            let action: IdentityUpdateTransitionAction = state_transition.into();
            return Ok(action.into());
        }

        let stored_identity = match maybe_stored_identity {
            None => {
                validation_result.add_error(SignatureError::IdentityNotFoundError(
                    IdentityNotFoundError::new(state_transition.get_identity_id().to_owned()),
                ));
                return Ok(validation_result);
            }
            Some(identity) => identity,
        };

        // copy identity
        let mut identity = stored_identity.clone();

        // Check revision
        if identity.get_revision() != (state_transition.get_revision() - 1) {
            validation_result.add_error(StateError::InvalidIdentityRevisionError {
                identity_id: state_transition.get_identity_id().to_owned(),
                current_revision: identity.get_revision(),
            });
            return Ok(validation_result);
        }

        for key_id in state_transition.get_public_key_ids_to_disable().iter() {
            match identity.get_public_key_by_id(*key_id) {
                None => {
                    validation_result
                        .add_error(StateError::InvalidIdentityPublicKeyIdError { id: *key_id });
                }
                Some(public_key_to_disable) => {
                    if public_key_to_disable.read_only {
                        validation_result.add_error(StateError::IdentityPublicKeyIsReadOnlyError {
                            public_key_index: *key_id,
                        })
                    }
                    if public_key_to_disable.is_disabled() {
                        validation_result.add_error(StateError::IdentityPublicKeyIsDisabledError {
                            public_key_index: *key_id,
                        })
                    }
                }
            }
        }
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        if !state_transition.get_public_key_ids_to_disable().is_empty() {
            let last_block_header_time = self
                .state_repository
                .fetch_latest_platform_block_time()
                .await
                .map_err(|e| {
                    NonConsensusError::StateRepositoryFetchError(format!(
                        "state repository fetch latest platform block time error: {}",
                        e.to_string()
                    ))
                })?;

            let disabled_at_ms = state_transition.get_public_keys_disabled_at().ok_or(
                NonConsensusError::RequiredPropertyError {
                    property_name: property_names::PUBLIC_KEYS_DISABLED_AT.to_owned(),
                },
            )?;
            let window_validation_result =
                validate_time_in_block_time_window(last_block_header_time, disabled_at_ms);

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

            // Keys can only be disabled if another valid key is enabled in the same security level
            for key_id in state_transition.get_public_key_ids_to_disable().iter() {
                let key =
                    identity
                        .get_public_key_by_id_mut(*key_id)
                        .ok_or(NonConsensusError::Error(anyhow!(
                            "public key must be present since it already validated during basic/stateless validation"
                        )))?;

                key.set_disabled_at(disabled_at_ms);
            }
        }

        identity.add_public_keys(
            state_transition
                .get_public_keys_to_add()
                .iter()
                .cloned()
                .map(|k| k.to_identity_public_key()),
        );

        let raw_public_keys = identity
            .public_keys
            .values()
            .map(|pk| pk.try_into().map_err(NonConsensusError::ValueError))
            .collect::<Result<Vec<Value>, NonConsensusError>>()?;

        let result = self
            .public_keys_validator
            .validate_keys(raw_public_keys.as_slice())?;
        if !result.is_valid() {
            validation_result.add_errors(result.errors);
            return Ok(validation_result);
        }

        let validator = RequiredPurposeAndSecurityLevelValidator {};
        let result = validator.validate_keys(&raw_public_keys)?;
        if !result.is_valid() {
            validation_result.add_errors(result.errors);
            return Ok(validation_result);
        }

        let action: IdentityUpdateTransitionAction = state_transition.into();
        Ok(action.into())
    }
}

use anyhow::anyhow;
use platform_value::Value;
use std::convert::TryInto;
use std::sync::Arc;

use crate::consensus::signature::IdentityNotFoundError;
use crate::consensus::signature::SignatureError;
use crate::consensus::state::identity::identity_public_key_disabled_at_window_violation_error::IdentityPublicKeyDisabledAtWindowViolationError;
use crate::consensus::state::identity::identity_public_key_is_disabled_error::IdentityPublicKeyIsDisabledError;
use crate::consensus::state::identity::identity_public_key_is_read_only_error::IdentityPublicKeyIsReadOnlyError;
use crate::consensus::state::identity::invalid_identity_public_key_id_error::InvalidIdentityPublicKeyIdError;
use crate::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
use crate::consensus::state::state_error::StateError;
use crate::identity::state_transition::identity_update_transition::IdentityUpdateTransitionAction;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::{
    block_time_window::validate_time_in_block_time_window::validate_time_in_block_time_window,
    identity::validation::{RequiredPurposeAndSecurityLevelValidator, TPublicKeysValidator},
    state_repository::StateRepositoryLike,
    validation::ConsensusValidationResult,
    NonConsensusError,
};
use crate::validation::block_time_window::validate_time_in_block_time_window::v0::validate_time_in_block_time_window_v0;

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
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<IdentityUpdateTransitionAction>, NonConsensusError> {
        let mut validation_result = ConsensusValidationResult::default();

        let maybe_stored_identity = self
            .state_repository
            .fetch_identity(state_transition.get_identity_id(), Some(execution_context))
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)
            .map_err(|e| {
                NonConsensusError::StateRepositoryFetchError(format!(
                    "state repository fetch identity for identity update validation error: {}",
                    e
                ))
            })?;

        if execution_context.is_dry_run() {
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
        if identity.get_revision()
            != (state_transition.get_revision().checked_sub(1).ok_or(
                NonConsensusError::Overflow("unable subtract 1 from revision"),
            )?)
        {
            validation_result.add_error(StateError::InvalidIdentityRevisionError(
                InvalidIdentityRevisionError::new(
                    state_transition.get_identity_id().to_owned(),
                    identity.get_revision(),
                ),
            ));
            return Ok(validation_result);
        }

        for key_id in state_transition.get_public_key_ids_to_disable().iter() {
            match identity.get_public_key_by_id(*key_id) {
                None => {
                    validation_result.add_error(StateError::InvalidIdentityPublicKeyIdError(
                        InvalidIdentityPublicKeyIdError::new(*key_id),
                    ));
                }
                Some(public_key_to_disable) => {
                    if public_key_to_disable.read_only {
                        validation_result.add_error(StateError::IdentityPublicKeyIsReadOnlyError(
                            IdentityPublicKeyIsReadOnlyError::new(*key_id),
                        ))
                    }
                    if public_key_to_disable.is_disabled() {
                        validation_result.add_error(StateError::IdentityPublicKeyIsDisabledError(
                            IdentityPublicKeyIsDisabledError::new(*key_id),
                        ))
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
                        e
                    ))
                })?;

            let disabled_at_ms = state_transition.get_public_keys_disabled_at().ok_or(
                NonConsensusError::RequiredPropertyError {
                    property_name: property_names::PUBLIC_KEYS_DISABLED_AT.to_owned(),
                },
            )?;
            //todo: add block spacing ms
            let window_validation_result =
                validate_time_in_block_time_window(last_block_header_time, disabled_at_ms, 0)?;

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

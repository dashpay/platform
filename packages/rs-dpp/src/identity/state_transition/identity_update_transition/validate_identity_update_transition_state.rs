use dashcore::BlockHeader;
use serde_json::Value;
use std::sync::Arc;

use crate::{
    block_time_window::validate_time_in_block_time_window::validate_time_in_block_time_window,
    consensus::basic::BasicError,
    identity::validation::{RequiredPurposeAndSecurityLevelValidator, TPublicKeysValidator},
    prelude::Identity,
    state_repository::StateRepositoryLike,
    state_transition::StateTransitionLike,
    validation::SimpleValidationResult,
    NonConsensusError, SerdeParsingError, StateError,
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
    ) -> Result<SimpleValidationResult, NonConsensusError> {
        let mut validation_result = SimpleValidationResult::default();

        let maybe_stored_identity: Option<Identity> = self
            .state_repository
            .fetch_identity(
                state_transition.get_identity_id(),
                state_transition.get_execution_context(),
            )
            .await
            .map_err(|e| NonConsensusError::StateRepositoryFetchError(e.to_string()))?;

        if state_transition.get_execution_context().is_dry_run() {
            return Ok(validation_result);
        }

        let stored_identity = match maybe_stored_identity {
            None => {
                validation_result.add_error(BasicError::IdentityNotFoundError {
                    identity_id: state_transition.get_identity_id().to_owned(),
                });
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
                        validation_result.add_error(StateError::IdentityPublicKeyDisabledError {
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
            // Keys can only be disabled if another valid key is enabled in the same security level
            for key_id in state_transition.get_public_key_ids_to_disable().iter() {
                // the `unwrap()` can be used as the presence if of `key_id` is guaranteed by previous
                // validation
                identity
                    .get_public_key_by_id_mut(*key_id)
                    .unwrap()
                    .disabled_at = state_transition.get_public_keys_disabled_at();
            }

            let block_header: BlockHeader = self
                .state_repository
                .fetch_latest_platform_block_header()
                .await
                .map_err(|e| NonConsensusError::StateRepositoryFetchError(e.to_string()))?;

            let last_block_header_time = block_header.time as u64 * 1000;
            let disabled_at_time = state_transition.get_public_keys_disabled_at().ok_or(
                NonConsensusError::RequiredPropertyError {
                    property_name: property_names::PUBLIC_KEYS_DISABLED_AT.to_owned(),
                },
            )?;
            let window_validation_result =
                validate_time_in_block_time_window(last_block_header_time, disabled_at_time);

            if !window_validation_result.is_valid() {
                validation_result.add_error(
                    StateError::IdentityPublicKeyDisabledAtWindowViolationError {
                        disabled_at: disabled_at_time,
                        time_window_start: window_validation_result.time_window_start,
                        time_window_end: window_validation_result.time_window_end,
                    },
                );
                return Ok(validation_result);
            }
        }

        let raw_public_keys: Vec<Value> = identity
            .public_keys
            .iter()
            .map(|pk| pk.to_raw_json_object(false))
            .collect::<Result<_, SerdeParsingError>>()?;

        if !state_transition.get_public_keys_to_add().is_empty() {
            identity.add_public_keys(state_transition.get_public_keys_to_add().iter().cloned());

            let result = self.public_keys_validator.validate_keys(&raw_public_keys)?;
            if !result.is_valid() {
                return Ok(result);
            }
        }

        let validator = RequiredPurposeAndSecurityLevelValidator {};
        let result = validator.validate_keys(&raw_public_keys)?;

        Ok(result)
    }
}

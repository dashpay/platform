use std::collections::{HashMap, HashSet};
use dpp::consensus::basic::identity::{DuplicatedIdentityPublicKeyIdBasicError, InvalidIdentityUpdateTransitionDisableKeysError, InvalidIdentityUpdateTransitionEmptyError};
use dpp::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::execution::validation::state_transition::common::validate_identity_public_keys_structure::v0::validate_identity_public_keys_structure_v0;

const MAX_KEYS_TO_DISABLE: usize = 10;

pub(crate) trait StateTransitionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionStructureValidationV0 for IdentityUpdateTransition {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let mut result = SimpleConsensusValidationResult::default();

        // Ensure that either disablePublicKeys or addPublicKeys is present
        if self.public_key_ids_to_disable().is_empty() && self.public_keys_to_add().is_empty() {
            result.add_error(InvalidIdentityUpdateTransitionEmptyError::new().into());
        }

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate public keys to disable
        if !self.public_key_ids_to_disable().is_empty() {
            // Ensure max items
            if self.public_key_ids_to_disable().len() > MAX_KEYS_TO_DISABLE {
                result.add_error(
                    MaxIdentityPublicKeyLimitReachedError::new(MAX_KEYS_TO_DISABLE).into(),
                );
            }

            // Check key id duplicates
            let mut ids = HashSet::new();
            for key_id in self.public_key_ids_to_disable() {
                if ids.contains(key_id) {
                    result.add_error(
                        DuplicatedIdentityPublicKeyIdBasicError::new(vec![key_id.clone()]).into(),
                    );
                    break;
                }

                ids.insert(key_id);
            }

            // Ensure disable at timestamp is present
            if self.public_keys_disabled_at().is_none() {
                result.add_error(InvalidIdentityUpdateTransitionDisableKeysError::new().into())
            }
        } else if self.public_keys_disabled_at().is_some() {
            // Ensure there are public keys to disable when disable at timestamp is present
            result.add_error(InvalidIdentityUpdateTransitionDisableKeysError::new().into())
        }

        if !result.is_valid() {
            return Ok(result);
        }

        validate_identity_public_keys_structure_v0(
            self.public_keys_to_add(),
            platform_version,
        )
    }
}

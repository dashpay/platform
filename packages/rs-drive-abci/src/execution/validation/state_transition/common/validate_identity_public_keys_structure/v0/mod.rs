use crate::error::Error;
use dpp::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyIdBasicError, InvalidIdentityPublicKeySecurityLevelError,
};
use dpp::consensus::basic::BasicError;

use dpp::consensus::state::identity::duplicated_identity_public_key_state_error::DuplicatedIdentityPublicKeyStateError;
use dpp::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;

use dpp::consensus::state::state_error::StateError;

use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

const MAX_PUBLIC_KEYS: usize = 10;

/// This validation will validate the count of new keys, that there are no duplicates either by
/// id or by data. This is done before signature and state validation to remove potential
/// attack vectors.
pub(crate) fn validate_identity_public_keys_structure_v0(
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    if identity_public_keys_with_witness.len() > MAX_PUBLIC_KEYS {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            StateError::MaxIdentityPublicKeyLimitReachedError(
                MaxIdentityPublicKeyLimitReachedError::new(MAX_PUBLIC_KEYS),
            )
            .into(),
        ));
    }

    // Check that there's not duplicates key ids in the state transition
    let duplicated_ids = IdentityPublicKeyInCreation::duplicated_key_ids_witness(
        identity_public_keys_with_witness,
        platform_version,
    )?;
    if !duplicated_ids.is_empty() {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            BasicError::DuplicatedIdentityPublicKeyIdBasicError(
                DuplicatedIdentityPublicKeyIdBasicError::new(duplicated_ids),
            )
            .into(),
        ));
    }

    // Check that there's no duplicated keys
    let duplicated_key_ids = IdentityPublicKeyInCreation::duplicated_keys_witness(
        identity_public_keys_with_witness,
        platform_version,
    )?;
    if !duplicated_key_ids.is_empty() {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            StateError::DuplicatedIdentityPublicKeyStateError(
                DuplicatedIdentityPublicKeyStateError::new(duplicated_key_ids),
            )
            .into(),
        ));
    }

    // We should check all the security levels
    let validation_errors = identity_public_keys_with_witness
        .iter()
        .filter_map(|identity_public_key| {
            let allowed_security_levels = ALLOWED_SECURITY_LEVELS.get(&identity_public_key.purpose);
            if let Some(levels) = allowed_security_levels {
                if !levels.contains(&identity_public_key.security_level()) {
                    Some(
                        InvalidIdentityPublicKeySecurityLevelError::new(
                            identity_public_key.id(),
                            identity_public_key.purpose(),
                            identity_public_key.security_level(),
                            Some(levels.clone()),
                        )
                        .into(),
                    )
                } else {
                    None //No error
                }
            } else {
                Some(
                    InvalidIdentityPublicKeySecurityLevelError::new(
                        identity_public_key.id(),
                        identity_public_key.purpose(),
                        identity_public_key.security_level(),
                        None,
                    )
                    .into(),
                )
            }
        })
        .collect();
    Ok(SimpleConsensusValidationResult::new_with_errors(
        validation_errors,
    ))
}

use crate::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyBasicError, DuplicatedIdentityPublicKeyIdBasicError,
    InvalidIdentityPublicKeySecurityLevelError, MissingMasterPublicKeyError,
    TooManyMasterPublicKeyError,
};
use crate::consensus::basic::BasicError;
use lazy_static::lazy_static;
use std::collections::HashMap;

use crate::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;

use crate::consensus::state::state_error::StateError;
use crate::identity::{Purpose, SecurityLevel};

use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

const MAX_PUBLIC_KEYS: usize = 6;

lazy_static! {
    static ref ALLOWED_SECURITY_LEVELS: HashMap<Purpose, Vec<SecurityLevel>> = {
        let mut m = HashMap::new();
        m.insert(
            Purpose::AUTHENTICATION,
            vec![
                SecurityLevel::MASTER,
                SecurityLevel::CRITICAL,
                SecurityLevel::HIGH,
                SecurityLevel::MEDIUM,
            ],
        );
        m.insert(Purpose::ENCRYPTION, vec![SecurityLevel::MEDIUM]);
        m.insert(Purpose::DECRYPTION, vec![SecurityLevel::MEDIUM]);
        m.insert(Purpose::TRANSFER, vec![SecurityLevel::CRITICAL]);
        m
    };
}
impl IdentityPublicKeyInCreation {
    /// This validation will validate the count of new keys, that there are no duplicates either by
    /// id or by data. This is done before signature and state validation to remove potential
    /// attack vectors.
    #[inline(always)]
    pub(super) fn validate_identity_public_keys_structure_v0(
        identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
        in_create_identity: bool,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
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
                BasicError::DuplicatedIdentityPublicKeyBasicError(
                    DuplicatedIdentityPublicKeyBasicError::new(duplicated_key_ids),
                )
                .into(),
            ));
        }

        if in_create_identity {
            // We should check that we are only adding one master authentication key

            let master_key_count = identity_public_keys_with_witness
                .iter()
                .filter(|key| key.security_level() == SecurityLevel::MASTER)
                .count();
            if master_key_count == 0 {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    BasicError::MissingMasterPublicKeyError(MissingMasterPublicKeyError::new())
                        .into(),
                ));
            } else if master_key_count > 1 {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    BasicError::TooManyMasterPublicKeyError(TooManyMasterPublicKeyError::new())
                        .into(),
                ));
            }
        }

        // We should check all the security levels
        let validation_errors = identity_public_keys_with_witness
            .iter()
            .filter_map(|identity_public_key| {
                let allowed_security_levels =
                    ALLOWED_SECURITY_LEVELS.get(&identity_public_key.purpose());
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
}

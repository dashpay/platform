use crate::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyBasicError, DuplicatedIdentityPublicKeyIdBasicError,
    InvalidIdentityPublicKeySecurityLevelError, InvalidKeyPurposeKeyTypeError,
    MissingMasterPublicKeyError, TooManyMasterPublicKeyError, TooManyPublicKeysOfPurposeError,
};
use crate::consensus::basic::BasicError;
use lazy_static::lazy_static;
use std::collections::HashMap;

use crate::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;

use crate::consensus::state::state_error::StateError;
use crate::identity::{KeyType, Purpose, SecurityLevel};

use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

lazy_static! {
    static ref ALLOWED_SECURITY_LEVELS_FOR_EXTERNALLY_ADDED_KEYS: HashMap<Purpose, Vec<SecurityLevel>> = {
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
        // DIP-33 payment detection/spend keys: non-signing keys, like
        // encryption/decryption
        m.insert(Purpose::PAYMENT_SCAN, vec![SecurityLevel::MEDIUM]);
        m.insert(Purpose::PAYMENT_SPEND, vec![SecurityLevel::MEDIUM]);
        m
    };
}

impl IdentityPublicKeyInCreation {
    /// This validation will validate the count of new keys, that there are no duplicates either by
    /// id or by data. This is done before signature and state validation to remove potential
    /// attack vectors.
    ///
    /// v1 (protocol version 14): accepts the DIP-33 `PAYMENT_SCAN` and
    /// `PAYMENT_SPEND` purposes, which must be `ECDSA_SECP256K1` and may appear
    /// at most once each per transition (the at-most-one-active-in-state rule
    /// for identity updates is enforced in state validation).
    #[inline(always)]
    pub(super) fn validate_identity_public_keys_structure_v1(
        identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
        in_create_identity: bool,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if identity_public_keys_with_witness.len()
            > platform_version
                .dpp
                .state_transitions
                .identities
                .max_public_keys_in_creation as usize
        {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                StateError::MaxIdentityPublicKeyLimitReachedError(
                    MaxIdentityPublicKeyLimitReachedError::new(
                        platform_version
                            .dpp
                            .state_transitions
                            .identities
                            .max_public_keys_in_creation as usize,
                    ),
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

        // DIP-33 payment keys: at most one of each payment purpose per
        // transition (on create this is also the at-most-one-active rule; on
        // update the against-state half lives in state validation), and only
        // ECDSA_SECP256K1 (the stealth derivation is defined over secp256k1
        // with the full compressed key published)
        for payment_purpose in Purpose::payment_purposes() {
            let payment_key_count = identity_public_keys_with_witness
                .iter()
                .filter(|key| key.purpose() == payment_purpose)
                .count();
            if payment_key_count > 1 {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    BasicError::TooManyPublicKeysOfPurposeError(
                        TooManyPublicKeysOfPurposeError::new(payment_purpose, 1),
                    )
                    .into(),
                ));
            }
        }

        if let Some(invalid_type_key) = identity_public_keys_with_witness.iter().find(|key| {
            Purpose::payment_purposes().contains(&key.purpose())
                && key.key_type() != KeyType::ECDSA_SECP256K1
        }) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                BasicError::InvalidKeyPurposeKeyTypeError(InvalidKeyPurposeKeyTypeError::new(
                    invalid_type_key.id(),
                    invalid_type_key.purpose(),
                    invalid_type_key.key_type(),
                    vec![KeyType::ECDSA_SECP256K1],
                ))
                .into(),
            ));
        }

        // We should check all the security levels
        let validation_errors = identity_public_keys_with_witness
            .iter()
            .filter_map(|identity_public_key| {
                let allowed_security_levels = ALLOWED_SECURITY_LEVELS_FOR_EXTERNALLY_ADDED_KEYS
                    .get(&identity_public_key.purpose());
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

#[cfg(test)]
mod tests {
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
    use crate::version::PlatformVersion;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn payment_key(
        id: u32,
        purpose: Purpose,
        key_type: KeyType,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> IdentityPublicKeyInCreation {
        let (key, _priv) = crate::identity::IdentityPublicKey::random_key_with_known_attributes(
            id,
            rng,
            purpose,
            SecurityLevel::MEDIUM,
            key_type,
            None,
            platform_version,
        )
        .expect("random payment key");
        key.into()
    }

    fn latest() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    #[test]
    fn accepts_a_single_ecdsa_payment_scan_key() {
        let platform_version = latest();
        let mut rng = StdRng::seed_from_u64(1);
        let keys = vec![payment_key(
            0,
            Purpose::PAYMENT_SCAN,
            KeyType::ECDSA_SECP256K1,
            &mut rng,
            platform_version,
        )];
        let result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure_v1(
            &keys,
            false,
            platform_version,
        )
        .expect("validation ran");
        assert!(result.is_valid(), "errors: {:?}", result.errors);
    }

    #[test]
    fn rejects_non_ecdsa_payment_key() {
        let platform_version = latest();
        let mut rng = StdRng::seed_from_u64(2);
        let keys = vec![payment_key(
            0,
            Purpose::PAYMENT_SPEND,
            KeyType::BLS12_381,
            &mut rng,
            platform_version,
        )];
        let result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure_v1(
            &keys,
            false,
            platform_version,
        )
        .expect("validation ran");
        assert!(matches!(
            result.errors.first(),
            Some(ConsensusError::BasicError(
                BasicError::InvalidKeyPurposeKeyTypeError(_)
            ))
        ));
    }

    #[test]
    fn rejects_two_keys_of_the_same_payment_purpose() {
        let platform_version = latest();
        let mut rng = StdRng::seed_from_u64(3);
        let keys = vec![
            payment_key(
                0,
                Purpose::PAYMENT_SCAN,
                KeyType::ECDSA_SECP256K1,
                &mut rng,
                platform_version,
            ),
            payment_key(
                1,
                Purpose::PAYMENT_SCAN,
                KeyType::ECDSA_SECP256K1,
                &mut rng,
                platform_version,
            ),
        ];
        let result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure_v1(
            &keys,
            false,
            platform_version,
        )
        .expect("validation ran");
        assert!(matches!(
            result.errors.first(),
            Some(ConsensusError::BasicError(
                BasicError::TooManyPublicKeysOfPurposeError(_)
            ))
        ));
    }
}
